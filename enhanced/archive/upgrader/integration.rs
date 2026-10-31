pub(crate) mod core {
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub const ID: &str = "upgrader";
pub const CHANNEL: &str = "upgrader:main";
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub house_edge: f64,
    pub min_chance: f64,
    pub max_chance: f64,
    pub spin_ticks: u32,
    pub max_target_count: u32,
    pub attempt_limit: u32,
    pub attempt_window_seconds: u32,
    pub global_value_multiplier: f64,
    pub value_overrides: BTreeMap<String, f64>,
    pub value_multipliers: BTreeMap<String, f64>,
    pub blacklist: BTreeSet<String>,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            house_edge: 0.9,
            min_chance: 0.001,
            max_chance: 0.9,
            spin_ticks: 70,
            max_target_count: 64,
            attempt_limit: 0,
            attempt_window_seconds: 0,
            global_value_multiplier: 1.0,
            value_overrides: BTreeMap::new(),
            value_multipliers: BTreeMap::new(),
            blacklist: BTreeSet::new(),
        }
    }
}
impl Config {
    pub fn validate(&self) -> Result<(), String> {
        let range = |v: f64, a: f64, b: f64| v.is_finite() && (a..=b).contains(&v);
        if !range(self.house_edge, 0.01, 1.0)
            || !range(self.min_chance, 0.0, 1.0)
            || !range(self.max_chance, 0.01, 1.0)
            || self.min_chance > self.max_chance
            || !(20..=600).contains(&self.spin_ticks)
            || !(1..=64).contains(&self.max_target_count)
            || self.attempt_limit > 10000
            || self.attempt_window_seconds > 86400
            || !range(self.global_value_multiplier, 0.01, 1000.0)
        {
            return Err("Invalid Upgrader settings".into());
        }
        if self.value_overrides.len() + self.value_multipliers.len() + self.blacklist.len() > 20000
        {
            return Err("Too many item settings".into());
        }
        for (id, n) in self
            .value_overrides
            .iter()
            .chain(self.value_multipliers.iter())
        {
            if !valid_id(id) || !range(*n, 0.0001, 1e12) {
                return Err(format!("Invalid price for {id}"));
            }
        }
        if self.blacklist.iter().any(|id| !valid_id(id)) {
            return Err("Invalid item id".into());
        }
        Ok(())
    }
    pub fn chance(&self, input: f64, output: f64) -> f64 {
        if !input.is_finite() || !output.is_finite() || input <= 0.0 || output <= 0.0 {
            return 0.0;
        }
        (self.house_edge * input / output).clamp(self.min_chance, self.max_chance)
    }
    pub fn price(&self, id: &str, base: f64) -> f64 {
        base * self.global_value_multiplier * self.value_multipliers.get(id).copied().unwrap_or(1.0)
    }
    pub fn blocked(&self, id: &str) -> bool {
        built_in_blocked(id) || self.blacklist.contains(id)
    }
}
pub fn valid_id(id: &str) -> bool {
    id.len() <= 256
        && id.contains(':')
        && id
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"_:./-".contains(&c))
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Stack {
    pub item: String,
    pub count: u32,
    pub damage: u32,
    pub max_damage: u32,
}
impl Stack {
    pub fn value(&self, unit: f64) -> f64 {
        let durability = if self.max_damage > 0 {
            (1.0 - self.damage as f64 / self.max_damage as f64).max(0.05)
        } else {
            1.0
        };
        unit * self.count as f64 * durability
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Item {
    pub id: String,
    pub name: String,
    pub max_stack: u32,
    pub block: bool,
    pub base: f64,
}
#[derive(Clone, Debug)]
pub struct Recipe {
    pub result: String,
    pub count: u32,
    pub groups: Vec<Vec<String>>,
}
#[derive(Clone, Debug)]
pub struct Drop {
    pub item: String,
    pub mean: f64,
    pub error: f64,
}
pub fn prices(
    items: &[Item],
    recipes: &[Recipe],
    loot: &BTreeMap<String, Vec<Drop>>,
    config: &Config,
) -> BTreeMap<String, f64> {
    let fallback: BTreeMap<_, _> = items.iter().map(|i| (i.id.clone(), i.base)).collect();
    let mut table = BTreeMap::new();
    let mut fixed = BTreeSet::new();
    for i in items {
        if let Some(v) = config.value_overrides.get(&i.id) {
            table.insert(i.id.clone(), *v);
            fixed.insert(i.id.clone());
        } else if let Some(v) = anchor(&i.id) {
            table.insert(i.id.clone(), v);
        }
    }
    let raw = |t: &BTreeMap<String, f64>, id: &str| {
        t.get(id)
            .or_else(|| fallback.get(id))
            .copied()
            .unwrap_or(3.0)
    };
    let relax = |t: &mut BTreeMap<String, f64>, f: &BTreeSet<String>| {
        for _ in 0..16 {
            let mut changed = false;
            for r in recipes {
                if r.count == 0 || r.groups.is_empty() || f.contains(&r.result) {
                    continue;
                }
                let sum: f64 = r
                    .groups
                    .iter()
                    .map(|g| {
                        g.iter()
                            .filter_map(|s| t.get(s).copied())
                            .fold(f64::INFINITY, f64::min)
                    })
                    .sum();
                let cost = (sum / r.count as f64).max(1.0);
                if cost.is_finite() && t.get(&r.result).is_none_or(|v| cost < *v - 1e-6) {
                    t.insert(r.result.clone(), cost);
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
    };
    relax(&mut table, &fixed);
    for _ in 0..24 {
        let mut raised = 0;
        for (block, drops) in loot {
            if config.value_overrides.contains_key(block) {
                continue;
            }
            let floor: f64 = drops
                .iter()
                .map(|d| (d.mean + 2.0 * d.error) * raw(&table, &d.item))
                .sum();
            if floor > raw(&table, block) + 1e-6 {
                table.insert(block.clone(), floor);
                fixed.insert(block.clone());
                raised += 1;
            }
        }
        let mut floors: BTreeMap<String, f64> = BTreeMap::new();
        for r in recipes {
            let Some(result) = table.get(&r.result) else {
                continue;
            };
            for id in r.groups.iter().flatten().collect::<BTreeSet<_>>() {
                if table.contains_key(id) {
                    continue;
                }
                let slots = r.groups.iter().filter(|g| g.contains(id)).count();
                if slots == 0 {
                    continue;
                }
                let rest: f64 = r
                    .groups
                    .iter()
                    .filter(|g| !g.contains(id))
                    .map(|g| {
                        g.iter()
                            .map(|s| raw(&table, s))
                            .fold(f64::INFINITY, f64::min)
                    })
                    .sum();
                let share = (*result * r.count as f64 - rest) / slots as f64;
                if share > fallback.get(id).copied().unwrap_or(3.0) + 1e-6 {
                    floors
                        .entry(id.clone())
                        .and_modify(|v| *v = v.max(share))
                        .or_insert(share);
                }
            }
        }
        raised += floors.len();
        for (id, v) in floors {
            table.insert(id.clone(), v);
            fixed.insert(id);
        }
        if raised == 0 {
            break;
        }
        relax(&mut table, &fixed);
    }
    for i in items {
        table.entry(i.id.clone()).or_insert(i.base);
    }
    table
}
#[derive(Default, Debug)]
pub struct Limiter {
    pub attempts: VecDeque<u64>,
}
impl Limiter {
    pub fn remaining(&mut self, c: &Config, now: u64) -> u32 {
        if c.attempt_limit == 0 || c.attempt_window_seconds == 0 {
            return 0;
        }
        let window = c.attempt_window_seconds as u64 * 1000;
        while self
            .attempts
            .front()
            .is_some_and(|t| t.saturating_add(window) <= now)
        {
            self.attempts.pop_front();
        }
        if self.attempts.len() < c.attempt_limit as usize {
            return 0;
        }
        self.attempts.front().map_or(0, |t| {
            t.saturating_add(window).saturating_sub(now).div_ceil(1000) as u32
        })
    }
    pub fn record(&mut self, c: &Config, now: u64) {
        if c.attempt_limit > 0 && c.attempt_window_seconds > 0 {
            self.attempts.push_back(now);
        }
    }
}
pub fn landing(chance: f64, won: bool, random: f64) -> f64 {
    let zone = chance * 360.0;
    let span = if won { zone } else { 360.0 - zone };
    let margin = 1.5f64.min(span * 0.2);
    let offset = margin + random.clamp(0.0, 1.0 - f64::EPSILON) * (span - 2.0 * margin).max(0.01);
    if won { offset } else { (zone + offset) % 360.0 }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum Request {
    Open,
    Clear,
    Select { slot: usize, count: Option<u32> },
    SelectHalf { slot: usize },
    Refresh,
    Target { item: String, count: u32 },
    Start,
    Config,
    Save { config: Config },
    Close,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct State {
    pub input: Option<Stack>,
    pub selected: Option<usize>,
    pub target: String,
    pub count: u32,
    pub chance: f64,
    pub cooldown: u32,
    pub error: String,
    pub result: Option<Spin>,
    pub inventory: Vec<Option<Stack>>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Spin {
    pub won: bool,
    pub angle: f64,
    pub ticks: u32,
    pub chance: f64,
    pub nonce: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Reply {
    Catalog {
        items: Vec<Item>,
        config: Config,
        can_edit: bool,
    },
    State {
        state: State,
    },
    Error {
        message: String,
    },
}

// Price anchors and the built-in deny list are generated from the supplied mod's metadata.
pub fn anchor(id: &str) -> Option<f64> {
    match id.strip_prefix("minecraft:").unwrap_or(id) {
        "dirt" | "cobblestone" | "cobbled_deepslate" | "sand" | "red_sand" | "gravel"
        | "netherrack" | "end_stone" | "granite" | "diorite" | "andesite" | "tuff" | "calcite"
        | "basalt" | "blackstone" | "snowball" | "wheat_seeds" | "kelp" => Some(1.0),
        "stone" | "deepslate" | "flint" | "bamboo" | "cactus" | "sugar_cane" | "obsidian"
        | "soul_sand" | "soul_soil" | "clay_ball" | "moss_block" => Some(2.0),
        "oak_log" | "spruce_log" | "birch_log" | "jungle_log" | "acacia_log" | "dark_oak_log"
        | "mangrove_log" | "cherry_log" | "crimson_stem" | "warped_stem" => Some(4.0),
        "wheat" | "potato" | "carrot" | "beetroot" | "apple" | "egg" => Some(3.0),
        "porkchop" | "beef" | "chicken" | "mutton" | "cod" | "salmon" => Some(6.0),
        "honeycomb" | "pufferfish" => Some(30.0),
        "chorus_fruit" => Some(4.0),
        "string" | "feather" | "bone" | "rotten_flesh" | "spider_eye" | "ink_sac" => Some(4.0),
        "leather" | "slime_ball" | "rabbit_hide" | "prismarine_shard" | "prismarine_crystals" => {
            Some(12.0)
        }
        "gunpowder" | "magma_cream" | "glow_ink_sac" => Some(20.0),
        "ender_pearl" | "blaze_rod" | "phantom_membrane" | "nautilus_shell" => Some(50.0),
        "ghast_tear" | "rabbit_foot" | "shulker_shell" => Some(120.0),
        "coal" => Some(6.0),
        "redstone" | "glowstone_dust" => Some(8.0),
        "lapis_lazuli" => Some(10.0),
        "copper_ingot" | "raw_copper" | "quartz" | "amethyst_shard" => Some(12.0),
        "iron_ingot" | "raw_iron" => Some(40.0),
        "gold_ingot" | "raw_gold" => Some(60.0),
        "emerald" => Some(140.0),
        "diamond" => Some(400.0),
        "ancient_debris" | "netherite_scrap" => Some(1200.0),
        "heart_of_the_sea" | "echo_shard" | "sponge" => Some(300.0),
        "saddle" | "name_tag" => Some(600.0),
        "netherite_upgrade_smithing_template" => Some(800.0),
        "totem_of_undying" | "trident" => Some(1500.0),
        "enchanted_golden_apple" => Some(2500.0),
        "elytra" => Some(2500.0),
        "nether_star" => Some(4000.0),
        "wither_skeleton_skull" => Some(1400.0),
        "dragon_egg" => Some(20000.0),
        _ => None,
    }
}
pub fn built_in_blocked(id: &str) -> bool {
    if id == "minecraft:air" || id.starts_with("upgrader:") || id.ends_with("_spawn_egg") {
        return true;
    }
    matches!(
        id,
        "minecraft:bedrock"
            | "minecraft:barrier"
            | "minecraft:light"
            | "minecraft:structure_block"
            | "minecraft:structure_void"
            | "minecraft:jigsaw"
            | "minecraft:command_block"
            | "minecraft:chain_command_block"
            | "minecraft:repeating_command_block"
            | "minecraft:command_block_minecart"
            | "minecraft:debug_stick"
            | "minecraft:knowledge_book"
            | "minecraft:spawner"
            | "minecraft:budding_amethyst"
            | "minecraft:end_portal_frame"
            | "minecraft:reinforced_deepslate"
            | "minecraft:petrified_oak_slab"
            | "minecraft:farmland"
            | "minecraft:dirt_path"
            | "minecraft:infested_stone"
            | "minecraft:infested_cobblestone"
            | "minecraft:infested_deepslate"
            | "minecraft:infested_stone_bricks"
            | "minecraft:infested_mossy_stone_bricks"
            | "minecraft:infested_cracked_stone_bricks"
            | "minecraft:infested_chiseled_stone_bricks"
            | "minecraft:trial_spawner"
            | "minecraft:vault"
            | "minecraft:suspicious_sand"
            | "minecraft:suspicious_gravel"
            | "minecraft:trial_key"
            | "minecraft:ominous_trial_key"
            | "minecraft:potion"
            | "minecraft:splash_potion"
            | "minecraft:lingering_potion"
            | "minecraft:tipped_arrow"
            | "minecraft:enchanted_book"
            | "minecraft:written_book"
            | "minecraft:suspicious_stew"
            | "minecraft:filled_map"
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_odds_and_durability() {
        let c = Config::default();
        assert_eq!(c.chance(40.0, 400.0), 0.09);
        assert_eq!(c.chance(1.0, 20000.0), 0.001);
        assert_eq!(c.chance(400.0, 1.0), 0.9);
        assert_eq!(c.chance(0.0, 1.0), 0.0);
        let s = Stack {
            count: 1,
            damage: 99,
            max_damage: 100,
            ..Default::default()
        };
        assert_eq!(s.value(100.0), 5.0);
    }
    #[test]
    fn rejects_nonfinite_and_invalid_settings() {
        let mut c = Config::default();
        c.house_edge = f64::NAN;
        assert!(c.validate().is_err());
        c = Config::default();
        c.min_chance = 0.95;
        assert!(c.validate().is_err());
    }
    #[test]
    fn sliding_window_expires_at_boundary() {
        let mut c = Config::default();
        c.attempt_limit = 2;
        c.attempt_window_seconds = 10;
        let mut l = Limiter::default();
        l.record(&c, 1000);
        l.record(&c, 2000);
        assert_eq!(l.remaining(&c, 2000), 9);
        assert_eq!(l.remaining(&c, 10999), 1);
        assert_eq!(l.remaining(&c, 11000), 0);
    }
    #[test]
    fn lands_inside_correct_sector() {
        for chance in [0.001, 0.09, 0.9] {
            for random in [0.0, 0.5, 0.999999] {
                assert!(landing(chance, true, random) < chance * 360.0);
                assert!(landing(chance, false, random) > chance * 360.0);
                assert!(landing(chance, false, random) < 360.0);
            }
        }
    }
    #[test]
    fn recipe_costs_and_fixed_overrides() {
        let i = |id: &str| Item {
            id: format!("minecraft:{id}"),
            name: id.into(),
            max_stack: 64,
            block: false,
            base: 3.0,
        };
        let items = vec![i("oak_log"), i("oak_planks"), i("stick")];
        let recipes = vec![
            Recipe {
                result: items[1].id.clone(),
                count: 4,
                groups: vec![vec![items[0].id.clone()]],
            },
            Recipe {
                result: items[2].id.clone(),
                count: 4,
                groups: vec![vec![items[1].id.clone()], vec![items[1].id.clone()]],
            },
        ];
        let mut c = Config::default();
        let p = prices(&items, &recipes, &BTreeMap::new(), &c);
        assert_eq!(p["minecraft:oak_planks"], 1.0);
        assert_eq!(p["minecraft:stick"], 1.0);
        c.value_overrides.insert("minecraft:stick".into(), 100.0);
        let p = prices(&items, &recipes, &BTreeMap::new(), &c);
        assert_eq!(p["minecraft:stick"], 100.0);
    }
    #[test]
    fn loot_floor_prevents_cheap_ore() {
        let items = vec![
            Item {
                id: "minecraft:diamond".into(),
                name: "Diamond".into(),
                max_stack: 64,
                block: false,
                base: 3.0,
            },
            Item {
                id: "minecraft:diamond_ore".into(),
                name: "Ore".into(),
                max_stack: 64,
                block: true,
                base: 3.0,
            },
        ];
        let loot = BTreeMap::from([(
            "minecraft:diamond_ore".into(),
            vec![Drop {
                item: "minecraft:diamond".into(),
                mean: 2.0,
                error: 0.1,
            }],
        )]);
        let p = prices(&items, &[], &loot, &Config::default());
        assert_eq!(p["minecraft:diamond_ore"], 880.0000000000001);
    }
    #[test]
    fn blocked_ids_and_namespace() {
        for id in [
            "minecraft:bedrock",
            "minecraft:pig_spawn_egg",
            "upgrader:upgrader",
        ] {
            assert!(built_in_blocked(id));
        }
        assert!(!built_in_blocked("minecraft:diamond"));
        assert!(!valid_id("../bad"));
    }
}

/// The visible remainder in a server inventory slot while part is selected.
pub fn unreserved(stack: &Stack, reserved: u32) -> Option<Stack> {
    let count = stack.count.saturating_sub(reserved);
    (count > 0).then(|| Stack {
        count,
        ..stack.clone()
    })
}
#[cfg(test)]
mod reservation_tests {
    use super::*;
    #[test]
    fn partial_selection_keeps_remainder_visible() {
        let s = Stack {
            item: "minecraft:iron_ingot".into(),
            count: 10,
            ..Default::default()
        };
        assert_eq!(unreserved(&s, 5).unwrap().count, 5);
        assert!(unreserved(&s, 10).is_none());
        assert!(unreserved(&s, 99).is_none());
    }
}

}
// Engine adapter pinned to Horizon 1da354b67a509812c4141b67de2affd6de65c342.
use self::core::*;
use super::{Game, Screen, lan::LanHost};
use horizon_client::{hud::HudBatch, items::ItemStack};
use horizon_core::random::{LegacyRandom, RandomSource};
use horizon_protocol::packets::{
    configuration::CustomPayload,
    play::{Clientbound as C, Serverbound as S},
};
use serde_json::Value;
use std::{
    cell::RefCell,
    collections::{BTreeMap, HashMap},
    sync::Arc,
    time::{Instant, SystemTime, UNIX_EPOCH},
};
use winit::{
    event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
};

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
fn stack(g: &Game<'_>, s: &ItemStack) -> Stack {
    Stack {
        item: s.item.to_string(),
        count: s.count,
        damage: s.damage,
        max_damage: g.res.items().get(&s.item).map_or(0, |i| i.max_damage),
    }
}
fn actual(s: &Stack) -> ItemStack {
    ItemStack {
        item: Arc::from(s.item.as_str()),
        count: s.count,
        damage: s.damage,
        extra: None,
    }
}
#[derive(Default)]
struct Session {
    selected: Option<usize>,
    stake: u32,
    expected: Option<ItemStack>,
    target: String,
    count: u32,
    limiter: Limiter,
    busy_until: u64,
    nonce: u64,
}
struct Server {
    config: Config,
    items: Vec<Item>,
    seeds: Vec<Item>,
    recipes: Vec<Recipe>,
    loot: BTreeMap<String, Vec<Drop>>,
    sessions: HashMap<u32, Session>,
}
thread_local! {static SERVERS:RefCell<HashMap<String,Server>>=RefCell::new(HashMap::new());static UI:RefCell<Ui>=RefCell::new(Ui::default());}
fn server(g: &mut Game<'_>) -> Server {
    let path = g.native_data_dir(ID).join("config.json");
    let config = std::fs::read(path)
        .ok()
        .and_then(|b| serde_json::from_slice::<Config>(&b).ok())
        .filter(|c| c.validate().is_ok())
        .unwrap_or_default();
    let data = g.native_recipes();
    let mut recipes = Vec::new();
    let mut items = Vec::new();
    let raw: Value = std::fs::read(g.options.resources.join("items.json"))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or(Value::Null);
    if let Some(data) = data {
        for i in &data.items {
            if let Some(info) = g.res.items().get(&i.id) {
                let rarity = raw
                    .get(&i.id)
                    .and_then(|v| v.get("minecraft:rarity"))
                    .and_then(Value::as_str)
                    .unwrap_or("common");
                items.push(Item {
                    id: i.id.clone(),
                    name: i.name.clone(),
                    max_stack: info.max_stack,
                    block: info.block.is_some(),
                    base: match rarity {
                        "uncommon" => 150.0,
                        "rare" => 600.0,
                        "epic" => 2500.0,
                        _ => 3.0,
                    },
                });
            }
        }
        for r in &data.recipes {
            if r.results.len() != 1 || r.slots.is_empty() {
                continue;
            }
            let result = &r.results[0];
            let Some(out) = data.items.get(result.item as usize) else {
                continue;
            };
            let mut count = result.count;
            let mut groups = Vec::new();
            for slot in &r.slots {
                let group: Vec<String> = slot
                    .iter()
                    .filter_map(|s| data.items.get(s.item as usize).map(|i| i.id.clone()))
                    .collect();
                if group.is_empty() {
                    continue;
                }
                if group == [out.id.clone()] {
                    count = count.saturating_sub(1)
                } else {
                    groups.push(group)
                }
            }
            if count > 0 && !groups.is_empty() {
                recipes.push(Recipe {
                    result: out.id.clone(),
                    count,
                    groups,
                });
            }
        }
    }
    // Reproduce the original 128 Fortune III block-drop samples and two-standard-error floor.
    let mut loot = BTreeMap::new();
    let mut random = LegacyRandom::new(0x55504752414445);
    let mut tool = ItemStack {
        item: Arc::from("minecraft:netherite_pickaxe"),
        count: 1,
        damage: 0,
        extra: Default::default(),
    };
    tool.extra
        .get_or_insert_with(Default::default)
        .enchantments
        .push((Arc::from("minecraft:fortune"), 3));
    for i in &items {
        if config.blocked(&i.id) {
            continue;
        }
        let Some(info) = g.res.items().get(&i.id) else {
            continue;
        };
        let Some(state) = info.block else { continue };
        let mut sums: BTreeMap<String, (f64, f64)> = BTreeMap::new();
        for _ in 0..128 {
            let drops =
                g.res
                    .items()
                    .block_drops(g.pack.assets.registry, state, Some(&tool), &mut random);
            let mut one: BTreeMap<String, u32> = BTreeMap::new();
            for s in drops {
                *one.entry(s.item.to_string()).or_default() += s.count;
            }
            for (id, n) in one {
                let e = sums.entry(id).or_default();
                e.0 += n as f64;
                e.1 += (n as f64).powi(2);
            }
        }
        let drops: Vec<Drop> = sums
            .into_iter()
            .filter_map(|(id, (sum, squares))| {
                let mean = sum / 128.0;
                if id == i.id && mean >= 1.0 {
                    return None;
                }
                Some(Drop {
                    item: id,
                    mean,
                    error: ((squares / 128.0 - mean * mean).max(0.0) / 128.0).sqrt(),
                })
            })
            .collect();
        if !drops.is_empty() {
            loot.insert(i.id.clone(), drops);
        }
    }
    let seeds = items.clone();
    let table = prices(&items, &recipes, &loot, &config);
    for i in &mut items {
        i.base = table[&i.id];
    }
    Server {
        config,
        items,
        seeds,
        recipes,
        loot,
        sessions: HashMap::new(),
    }
}
fn snapshot(g: &Game<'_>, s: &mut Session, c: &Config, items: &[Item], time: u64) -> State {
    let input = s
        .selected
        .and_then(|i| g.inventory.get(i))
        .and_then(Option::as_ref)
        .map(|a| {
            let mut v = stack(g, a);
            v.count = s.stake.min(a.count);
            v
        });
    let val = |id: &str| {
        items
            .iter()
            .find(|i| i.id == id)
            .map_or(0.0, |i| c.price(id, i.base))
    };
    let chance = c.chance(
        input.as_ref().map_or(0.0, |i| i.value(val(&i.item))),
        val(&s.target) * s.count.max(1) as f64,
    );
    State {
        input,
        selected: s.selected,
        target: s.target.clone(),
        count: s.count.max(1),
        chance,
        cooldown: s.limiter.remaining(c, time),
        inventory: g
            .inventory
            .iter()
            .take(36)
            .map(|s| s.as_ref().map(|s| stack(g, s)))
            .collect(),
        ..Default::default()
    }
}
fn handle(g: &mut Game<'_>, conn: u32, permission: i32, request: Request) -> Vec<Reply> {
    // Splitting is computed from the authoritative inventory, never a stale client mirror.
    let request = match request {
        Request::SelectHalf { slot } => Request::Select {
            slot,
            count: g
                .inventory
                .get(slot)
                .and_then(Option::as_ref)
                .map(|s| s.count.div_ceil(2)),
        },
        other => other,
    };
    let key = g.options.world.clone().unwrap_or_else(|| "server".into());
    SERVERS.with(|servers| {
        let mut servers = servers.borrow_mut();
        let sv = servers.entry(key).or_insert_with(|| server(g));
        let has_station = g
            .inventory
            .iter()
            .flatten()
            .any(|s| &*s.item == "upgrader:upgrader");
        let admin = permission >= 2;
        if !has_station
            && !matches!(
                request,
                Request::Config | Request::Save { .. } | Request::Close
            )
        {
            return vec![Reply::Error {
                message: "Нужен предмет Upgrader: скрафти его или возьми в творческом режиме."
                    .into(),
            }];
        }
        if matches!(request, Request::Config | Request::Open) {
            let c = sv.config.clone();
            let items = sv.items.clone();
            let s = sv.sessions.entry(conn).or_default();
            return vec![
                Reply::Catalog {
                    items: items.clone(),
                    config: c.clone(),
                    can_edit: admin,
                },
                Reply::State {
                    state: snapshot(g, s, &c, &items, now()),
                },
            ];
        }
        if let Request::Save { config } = request {
            if !admin {
                return vec![Reply::Error {
                    message: "Настройки сервера может менять только оператор.".into(),
                }];
            }
            if let Err(message) = config.validate() {
                return vec![Reply::Error { message }];
            }
            let path = g.native_data_dir(ID).join("config.json");
            let saved = std::fs::create_dir_all(path.parent().unwrap()).and_then(|_| {
                let tmp = path.with_extension("tmp");
                std::fs::write(&tmp, serde_json::to_vec_pretty(&config).unwrap())?;
                std::fs::rename(tmp, &path)
            });
            if let Err(e) = saved {
                return vec![Reply::Error {
                    message: format!("Не удалось сохранить: {e}"),
                }];
            }
            sv.config = config;
            let table = prices(&sv.seeds, &sv.recipes, &sv.loot, &sv.config);
            for i in &mut sv.items {
                i.base = table[&i.id];
            }
            return vec![Reply::Catalog {
                items: sv.items.clone(),
                config: sv.config.clone(),
                can_edit: true,
            }];
        }
        let s = sv.sessions.entry(conn).or_default();
        let time = now();
        let c = &sv.config;
        if time < s.busy_until && !matches!(request, Request::Close | Request::Refresh) {
            return vec![Reply::Error {
                message: "Колесо ещё вращается.".into(),
            }];
        }
        let mut spin = None;
        let mut error = String::new();
        match request {
            Request::Clear => {
                s.selected = None;
                s.expected = None;
            }
            Request::Select { slot, count } => {
                if slot >= 36 {
                    error = "Неверный слот".into()
                } else if let Some(item) = g.inventory[slot].as_ref() {
                    if c.blocked(&item.item) {
                        error = "Этот предмет запрещён".into()
                    } else {
                        s.selected = Some(slot);
                        s.stake = count.unwrap_or(item.count).clamp(1, item.count);
                        s.expected = Some(item.clone());
                    }
                } else {
                    s.selected = None;
                    s.expected = None;
                }
            }
            Request::Target { item, count } => {
                if c.blocked(&item) {
                    error = "Этот предмет запрещён".into()
                } else if let Some(i) = sv.items.iter().find(|i| i.id == item) {
                    s.target = item;
                    s.count = count.clamp(1, c.max_target_count.min(i.max_stack).max(1));
                }
            }
            Request::Start => {
                let cooldown = s.limiter.remaining(c, time);
                let input = s
                    .selected
                    .and_then(|i| g.inventory.get(i))
                    .and_then(Option::as_ref)
                    .cloned();
                let target = sv.items.iter().find(|i| i.id == s.target);
                if cooldown > 0 {
                    error = format!("Подожди {cooldown} сек.")
                } else if input.is_none() || s.expected != input {
                    error = "Предмет в инвентаре изменился. Выбери его заново.".into()
                } else if let (Some(input), Some(target), Some(slot)) = (input, target, s.selected)
                {
                    if c.blocked(&input.item) || c.blocked(&target.id) {
                        error = "Предмет запрещён".into()
                    } else {
                        let count = s
                            .count
                            .clamp(1, c.max_target_count.min(target.max_stack).max(1));
                        let input_unit = sv
                            .items
                            .iter()
                            .find(|i| i.id == &*input.item)
                            .map_or(0.0, |i| c.price(&i.id, i.base));
                        let chance = c.chance(
                            {
                                let mut v = stack(g, &input);
                                v.count = s.stake;
                                v.value(input_unit)
                            },
                            c.price(&target.id, target.base) * count as f64,
                        );
                        if chance <= 0.0 {
                            error = "Стоимость предметов не определена".into()
                        } else {
                            let won = g.world.random.next_double() < chance;
                            let angle = landing(chance, won, g.world.random.next_double());
                            // Settle atomically on the authoritative server. No item is escrowed in
                            // client memory: disconnecting during animation cannot lose or duplicate it.
                            let remaining = input.count.saturating_sub(s.stake);
                            g.inventory[slot] = if remaining > 0 {
                                Some(ItemStack {
                                    count: remaining,
                                    ..input
                                })
                            } else {
                                None
                            };
                            s.selected = None;
                            s.expected = None;
                            s.stake = 0;
                            if won {
                                let output = ItemStack {
                                    item: Arc::from(target.id.as_str()),
                                    count,
                                    damage: 0,
                                    extra: None,
                                };
                                if let Some(index) =
                                    g.inventory.iter().take(36).position(Option::is_none)
                                {
                                    g.inventory[index] = Some(output.clone());
                                    s.selected = Some(index);
                                    s.expected = Some(output);
                                    s.stake = count;
                                } else if let Some(rest) = g.give(output) {
                                    if let Some(p) = g.native_player() {
                                        g.pop_resource(
                                            [
                                                p.x.floor() as i32,
                                                p.y.floor() as i32,
                                                p.z.floor() as i32,
                                            ],
                                            rest,
                                        );
                                    }
                                }
                            }
                            s.limiter.record(c, time);
                            s.busy_until = time + c.spin_ticks as u64 * 50;
                            s.nonce += 1;
                            spin = Some(Spin {
                                won,
                                angle,
                                ticks: c.spin_ticks,
                                chance,
                                nonce: s.nonce,
                            });
                        }
                    }
                } else {
                    error = "Выбери исходный и целевой предметы".into()
                }
            }
            Request::Close => {
                s.selected = None;
                s.expected = None;
            }
            _ => {}
        }
        let mut state = snapshot(g, s, c, &sv.items, time);
        state.error = error;
        state.result = spin;
        vec![Reply::State { state }]
    })
}
pub fn guest(g: &mut Game<'_>, host: &mut LanHost, conn: u32, packet: &S) -> bool {
    let S::CustomPayload(p) = packet else {
        return false;
    };
    if p.channel != CHANNEL {
        return false;
    }
    if p.data.len() > 262144 {
        return true;
    }
    let Some(gi) = host.hub.guests.iter().position(|a| a.conn == conn) else {
        return true;
    };
    if host.hub.guests[gi].dead || host.hub.guests[gi].spectator() {
        return true;
    }
    let permission = if host.hub.singleplayer_owner == Some(host.hub.guests[gi].uuid) {
        4
    } else {
        i32::from(host.hub.guests[gi].permission)
    };
    let Ok(request) = serde_json::from_slice::<Request>(&p.data) else {
        return true;
    };
    let saving = matches!(&request, Request::Save { .. });
    let replies = g
        .as_guest(host, conn, |g| handle(g, conn, permission, request))
        .unwrap_or_default();
    for reply in replies {
        if saving {
            if let Reply::Catalog { items, config, .. } = &reply {
                let recipients: Vec<(u32, bool)> = host
                    .hub
                    .guests
                    .iter()
                    .map(|g| {
                        (
                            g.conn,
                            g.permission >= 2 || host.hub.singleplayer_owner == Some(g.uuid),
                        )
                    })
                    .collect();
                for (target, can_edit) in recipients {
                    let r = Reply::Catalog {
                        items: items.clone(),
                        config: config.clone(),
                        can_edit,
                    };
                    if let Ok(data) = serde_json::to_vec(&r) {
                        host.hub.send_to(
                            target,
                            &C::CustomPayload(CustomPayload {
                                channel: CHANNEL.into(),
                                data,
                            }),
                        );
                    }
                }
                continue;
            }
        }
        if let Ok(data) = serde_json::to_vec(&reply) {
            host.hub.send_to(
                conn,
                &C::CustomPayload(CustomPayload {
                    channel: CHANNEL.into(),
                    data,
                }),
            );
        }
    }
    true
}
pub fn received(packet: &C) -> bool {
    let C::CustomPayload(p) = packet else {
        return false;
    };
    if p.channel != CHANNEL {
        return false;
    }
    if p.data.len() > 1048576 {
        return true;
    }
    if let Ok(reply) = serde_json::from_slice::<Reply>(&p.data) {
        UI.with(|u| {
            let mut u = u.borrow_mut();
            u.waiting = false;
            match reply {
                Reply::Catalog {
                    items,
                    config,
                    can_edit,
                } => {
                    u.items = items;
                    u.config = config.clone();
                    u.draft = config;
                    u.can_edit = can_edit;
                    u.error.clear();
                }
                Reply::State { state } => {
                    if let Some(s) = &state.result {
                        u.spin_input = u.state.input.clone();
                        u.spin = Some((s.clone(), Instant::now()));
                        u.last_spin = Some(s.clone());
                    }
                    if !state.error.is_empty() {
                        u.error = state.error.clone();
                    }
                    u.state_time = Instant::now();
                    u.state = state;
                }
                Reply::Error { message } => u.error = message,
            }
        })
    }
    true
}
fn send(g: &mut Game<'_>, r: Request) {
    if g.remote.is_some() {
        if let Ok(data) = serde_json::to_vec(&r) {
            g.send(S::CustomPayload(CustomPayload {
                channel: CHANNEL.into(),
                data,
            }));
        }
        UI.with(|u| u.borrow_mut().waiting = true);
    } else {
        let replies = handle(g, 0, 4, r);
        for reply in replies {
            if let Ok(data) = serde_json::to_vec(&reply) {
                received(&C::CustomPayload(CustomPayload {
                    channel: CHANNEL.into(),
                    data,
                }));
            }
        }
    }
}
pub fn open(g: &mut Game<'_>) {
    if g.player.is_none() || !g.native_on(ID) {
        return;
    }
    UI.with(|u| {
        let mut u = u.borrow_mut();
        *u = Ui::default();
        u.open = true;
        u.opened = Instant::now();
    });
    g.close_container();
    g.keys.clear();
    g.physical_keys.clear();
    g.physical_buttons.clear();
    g.left = false;
    g.right = false;
    g.screen = Screen::Inventory;
    g.capture(false);
    send(g, Request::Open);
}
pub fn command(g: &mut Game<'_>, command: &str) -> bool {
    let args: Vec<&str> = command.split_whitespace().collect();
    if args.first() != Some(&"upgrader") {
        return false;
    }
    match args.as_slice(){
        [_]=>open(g),[_,"config"]=>settings_open(g),[_,"close"]=>close(g),
        [_,"select",slot]=>{if let Ok(slot)=slot.parse::<usize>(){send(g,Request::Select{slot,count:None});}},
        [_,"target",item,count]=>{if let Ok(count)=count.parse(){send(g,Request::Target{item:(*item).into(),count});}},
        [_,"start"]=>send(g,Request::Start),
        _=>g.native_chat("/upgrader | /upgrader config | /upgrader select 0..35 | /upgrader target minecraft:diamond 1 | /upgrader start"),
    }
    true
}
pub fn use_item(g: &mut Game<'_>, button: MouseButton, pressed: bool) -> bool {
    if pressed
        && button == MouseButton::Right
        && g.screen == Screen::Playing
        && g.inventory
            .get(g.selected)
            .and_then(Option::as_ref)
            .is_some_and(|s| &*s.item == "upgrader:upgrader")
    {
        open(g);
        true
    } else {
        false
    }
}
pub fn active() -> bool {
    UI.with(|u| u.borrow().open)
}
fn close(g: &mut Game<'_>) {
    UI.with(|u| u.borrow_mut().open = false);
    send(g, Request::Close);
    g.screen = Screen::Playing;
    g.capture(true);
}
#[derive(Clone, Copy, PartialEq, Default)]
enum Page {
    #[default]
    Main,
    Catalog,
    Settings,
    Prices,
}
#[derive(Clone, Copy)]
enum Edit {
    Search,
    Count,
    Setting(usize),
    Price(usize),
    Multiplier(usize),
}
struct Ui {
    open: bool,
    page: Page,
    items: Vec<Item>,
    config: Config,
    draft: Config,
    can_edit: bool,
    state: State,
    search: String,
    row: usize,
    filter: u8,
    sort: u8,
    focus: Option<Edit>,
    text: String,
    error: String,
    spin: Option<(Spin, Instant)>,
    last_spin: Option<Spin>,
    spin_input: Option<Stack>,
    played_nonce: u64,
    state_time: Instant,
    waiting: bool,
    opened: Instant,
    last_poll: Instant,
    drag: Option<usize>,
    size: [u32; 2],
    mouse: (f32, f32),
    hot: Vec<(Rect, Action)>,
}
impl Default for Ui {
    fn default() -> Self {
        Self {
            open: false,
            page: Page::Main,
            items: Vec::new(),
            config: Config::default(),
            draft: Config::default(),
            can_edit: false,
            state: State::default(),
            search: String::new(),
            row: 0,
            filter: 0,
            sort: 0,
            focus: None,
            text: String::new(),
            error: String::new(),
            spin: None,
            last_spin: None,
            spin_input: None,
            played_nonce: 0,
            state_time: Instant::now(),
            waiting: false,
            opened: Instant::now(),
            last_poll: Instant::now(),
            drag: None,
            size: [1280, 720],
            mouse: (0.0, 0.0),
            hot: Vec::new(),
        }
    }
}
#[derive(Clone, Copy)]
struct Rect(f32, f32, f32, f32);
impl Rect {
    fn has(self, x: f32, y: f32) -> bool {
        x >= self.0 && y >= self.1 && x < self.0 + self.2 && y < self.1 + self.3
    }
}
#[derive(Clone)]
enum Action {
    Close,
    Back,
    Settings,
    Catalog,
    Prices,
    Select(usize),
    SelectPart(usize),
    Target(String),
    Count(u32),
    Start,
    Focus(Edit, String),
    Filter,
    Sort,
    Previous,
    Next,
    Blacklist(String),
    Save,
    Clear,
}
const BG: u32 = 0xff11151c;
const CARD: u32 = 0xff1e2530;
const BORDER: u32 = 0xff394552;
const FG: u32 = 0xffe6eaf1;
const DIM: u32 = 0xff8795a9;
const GOLD: u32 = 0xffffca5c;
const SETTINGS: [(&str, &str); 8] = [
    ("House edge", "house_edge"),
    ("Minimum chance", "min_chance"),
    ("Maximum chance", "max_chance"),
    ("Spin ticks", "spin_ticks"),
    ("Max target count", "max_target_count"),
    ("Attempt limit", "attempt_limit"),
    ("Window (seconds)", "attempt_window_seconds"),
    ("Global multiplier", "global_value_multiplier"),
];
fn setting(c: &Config, i: usize) -> String {
    let v = serde_json::to_value(c).unwrap();
    v[SETTINGS[i].1].to_string()
}
fn edit_commit(u: &mut Ui) {
    let Some(f) = u.focus.take() else { return };
    let text = u.text.trim();
    match f {
        Edit::Search => {
            u.search = u.text.clone();
            u.row = 0
        }
        Edit::Count => {}
        Edit::Setting(i) => {
            let Ok(n) = text.parse::<f64>() else {
                u.error = "Введите число".into();
                return;
            };
            let mut json = serde_json::to_value(&u.draft).unwrap();
            json[SETTINGS[i].1] = if i >= 3 && i <= 6 {
                serde_json::json!(n as u32)
            } else {
                serde_json::json!(n)
            };
            if let Ok(c) = serde_json::from_value(json) {
                u.draft = c;
            }
        }
        Edit::Price(i) | Edit::Multiplier(i) => {
            let Some(item) = u.items.get(i) else { return };
            let id = item.id.clone();
            let map = if matches!(f, Edit::Price(_)) {
                &mut u.draft.value_overrides
            } else {
                &mut u.draft.value_multipliers
            };
            if text.is_empty() {
                map.remove(&id);
            } else if let Ok(n) = text.parse::<f64>() {
                if n.is_finite() && n > 0.0 {
                    map.insert(id, n);
                } else {
                    u.error = "Цена должна быть больше нуля".into();
                }
            } else {
                u.error = "Введите число".into();
            }
        }
    }
}
fn spinning(u: &Ui) -> bool {
    u.spin
        .as_ref()
        .is_some_and(|(s, t)| t.elapsed().as_millis() < s.ticks as u128 * 50)
}
fn fill(g: &Game<'_>, b: &mut HudBatch, r: Rect, c: u32) {
    g.native_fill(b, [r.0, r.1, r.2, r.3], c)
}
fn text(g: &Game<'_>, b: &mut HudBatch, s: &str, x: f32, y: f32, c: u32) {
    g.native_text(b, s, [x, y], c & 0xffffff, false)
}
fn centered(g: &Game<'_>, b: &mut HudBatch, s: &str, x: f32, y: f32, c: u32) {
    text(g, b, s, x - g.native_text_width(s) / 2.0, y, c)
}
fn panel(g: &Game<'_>, b: &mut HudBatch, r: Rect, c: u32) {
    fill(g, b, r, BORDER);
    fill(g, b, Rect(r.0 + 1.0, r.1 + 1.0, r.2 - 2.0, r.3 - 2.0), c)
}
fn button(
    g: &Game<'_>,
    b: &mut HudBatch,
    u: &mut Ui,
    r: Rect,
    label: &str,
    a: Action,
    enabled: bool,
) {
    let hovered = r.has(u.mouse.0, u.mouse.1);
    panel(g, b, r, if enabled && hovered { 0xff45546a } else { CARD });
    centered(
        g,
        b,
        label,
        r.0 + r.2 / 2.0,
        r.1 + (r.3 - 8.0) / 2.0,
        if enabled { FG } else { DIM },
    );
    if enabled {
        u.hot.push((r, a));
    }
}
fn field(g: &Game<'_>, b: &mut HudBatch, u: &mut Ui, r: Rect, value: &str, f: Edit, enabled: bool) {
    panel(g, b, r, 0xff0c1017);
    let active = u.focus.is_some_and(|a| match (a, f) {
        (Edit::Search, Edit::Search) | (Edit::Count, Edit::Count) => true,
        (Edit::Setting(a), Edit::Setting(b))
        | (Edit::Price(a), Edit::Price(b))
        | (Edit::Multiplier(a), Edit::Multiplier(b)) => a == b,
        _ => false,
    });
    let shown = if active {
        format!("{}|", u.text)
    } else {
        value.into()
    };
    let width = ((r.2 - 6.0) / 6.0).max(1.0) as usize;
    let clipped: String = shown
        .chars()
        .rev()
        .take(width)
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    text(
        g,
        b,
        &clipped,
        r.0 + 3.0,
        r.1 + 5.0,
        if enabled { FG } else { DIM },
    );
    if enabled {
        u.hot.push((r, Action::Focus(f, value.into())));
    }
}
fn item_slot(g: &Game<'_>, b: &mut HudBatch, r: Rect, s: Option<&Stack>, selected: bool) {
    panel(g, b, r, if selected { 0xff394044 } else { 0xff0b0f14 });
    if let Some(s) = s {
        g.draw_item(b, &actual(s), r.0 + 1.0, r.1 + 1.0);
    }
}
fn filtered(u: &Ui) -> Vec<usize> {
    let query = u.search.to_lowercase();
    let mut out: Vec<usize> = u
        .items
        .iter()
        .enumerate()
        .filter(|(_, i)| {
            let matching = i.id.contains(&query) || i.name.to_lowercase().contains(&query);
            let changed = u.draft.value_overrides.contains_key(&i.id)
                || u.draft.value_multipliers.contains_key(&i.id)
                || u.draft.blacklist.contains(&i.id);
            matching
                && match u.page {
                    Page::Catalog => !u.config.blocked(&i.id),
                    _ => match u.filter {
                        1 => i.block,
                        2 => !i.block,
                        3 => changed,
                        _ => true,
                    },
                }
        })
        .map(|(i, _)| i)
        .collect();
    out.sort_by(|a, b| {
        let (a, b) = (&u.items[*a], &u.items[*b]);
        match u.sort {
            1 => u
                .config
                .price(&a.id, a.base)
                .total_cmp(&u.config.price(&b.id, b.base)),
            2 => u
                .config
                .price(&b.id, b.base)
                .total_cmp(&u.config.price(&a.id, a.base)),
            _ => a.name.cmp(&b.name),
        }
        .then(a.id.cmp(&b.id))
    });
    out
}
fn wheel(g: &Game<'_>, b: &mut HudBatch, cx: f32, cy: f32, chance: f64, angle: f64) {
    for y in -42..=42 {
        for x in -42..=42 {
            let radius = ((x * x + y * y) as f64).sqrt();
            if (31.0..=41.0).contains(&radius) {
                let a = ((x as f64).atan2(-(y as f64)).to_degrees() + 360.0) % 360.0;
                fill(
                    g,
                    b,
                    Rect(cx + x as f32, cy + y as f32, 1.0, 1.0),
                    if a < chance * 360.0 { GOLD } else { 0xff313c4b },
                );
            }
        }
    }
    for j in 0..36 {
        let a = (j as f64 * 10.0).to_radians();
        let (x, y) = (a.sin() as f32, -a.cos() as f32);
        fill(g, b, Rect(cx + x * 45.0, cy + y * 45.0, 1.0, 2.0), DIM);
    }
    let a = angle.to_radians();
    for d in 4..34 {
        fill(
            g,
            b,
            Rect(
                cx + a.sin() as f32 * d as f32,
                cy - a.cos() as f32 * d as f32,
                2.0,
                2.0,
            ),
            GOLD,
        );
    }
    fill(g, b, Rect(cx - 2.0, cy - 2.0, 5.0, 5.0), FG);
}
pub fn draw(g: &mut Game<'_>, size: [u32; 2]) -> Option<HudBatch> {
    if !active() {
        return None;
    }
    if g.player.is_none() {
        UI.with(|u| u.borrow_mut().open = false);
        return None;
    }
    let mut b = HudBatch::new(size);
    let (bw, bh) = (b.width, b.height);
    UI.with(|ui| {
        let mut u = ui.borrow_mut();
        u.size = size;
        u.mouse = g.cursor;
        u.hot.clear();
        fill(g, &mut b, Rect(0.0, 0.0, bw, bh), 0xe810141b);
        let (w, h) = if matches!(u.page, Page::Main | Page::Catalog) {
            (288.0, 240.0)
        } else {
            (310.0, 236.0)
        };
        let (x, y) = ((b.width - w) / 2.0, (b.height - h) / 2.0);
        panel(g, &mut b, Rect(x, y, w, h), BG);
        let title = match u.page {
            Page::Main => "ITEM UPGRADER",
            Page::Catalog => "ВЫБЕРИ ЦЕЛЬ",
            Page::Settings => "UPGRADER SETTINGS",
            Page::Prices => "ЦЕНЫ ПРЕДМЕТОВ",
        };
        centered(g, &mut b, title, x + w / 2.0, y + 7.0, FG);
        button(
            g,
            &mut b,
            &mut u,
            Rect(x + w - 18.0, y + 3.0, 15.0, 14.0),
            "X",
            Action::Close,
            true,
        );
        match u.page {
            Page::Main => {
                let busy = spinning(&u);
                let input = if busy {
                    u.spin_input.clone()
                } else {
                    u.state.input.clone()
                };
                let target = u.items.iter().find(|i| i.id == u.state.target).cloned();
                panel(g, &mut b, Rect(x + 8.0, y + 22.0, 82.0, 94.0), CARD);
                panel(g, &mut b, Rect(x + 198.0, y + 22.0, 82.0, 94.0), CARD);
                item_slot(
                    g,
                    &mut b,
                    Rect(x + 40.0, y + 34.0, 18.0, 18.0),
                    input.as_ref(),
                    false,
                );
                if !busy {
                    u.hot
                        .push((Rect(x + 8.0, y + 22.0, 82.0, 94.0), Action::Clear));
                }
                if let Some(i) = &input {
                    let price = u
                        .items
                        .iter()
                        .find(|t| t.id == i.item)
                        .map_or(0.0, |t| u.config.price(&t.id, t.base));
                    centered(
                        g,
                        &mut b,
                        &format!("Value: {:.0}", i.value(price)),
                        x + 49.0,
                        y + 83.0,
                        FG,
                    );
                } else {
                    centered(g, &mut b, "Выбери предмет", x + 49.0, y + 60.0, DIM);
                    centered(g, &mut b, "из инвентаря", x + 49.0, y + 71.0, DIM);
                }
                let target_stack = target.as_ref().map(|i| Stack {
                    item: i.id.clone(),
                    count: u.state.count.max(1),
                    ..Default::default()
                });
                item_slot(
                    g,
                    &mut b,
                    Rect(x + 230.0, y + 34.0, 18.0, 18.0),
                    target_stack.as_ref(),
                    false,
                );
                if !busy {
                    u.hot
                        .push((Rect(x + 198.0, y + 22.0, 82.0, 54.0), Action::Catalog));
                }
                if let Some(i) = target {
                    centered(
                        g,
                        &mut b,
                        &format!(
                            "Value: {:.0}",
                            u.config.price(&i.id, i.base) * u.state.count.max(1) as f64
                        ),
                        x + 239.0,
                        y + 61.0,
                        FG,
                    );
                } else {
                    centered(g, &mut b, "Выбери цель", x + 239.0, y + 60.0, DIM);
                }
                let value = u.state.count.max(1).to_string();
                field(
                    g,
                    &mut b,
                    &mut u,
                    Rect(x + 210.0, y + 80.0, 57.0, 18.0),
                    &value,
                    Edit::Count,
                    !busy,
                );
                button(
                    g,
                    &mut b,
                    &mut u,
                    Rect(x + 201.0, y + 100.0, 24.0, 13.0),
                    "1",
                    Action::Count(1),
                    !busy,
                );
                button(
                    g,
                    &mut b,
                    &mut u,
                    Rect(x + 227.0, y + 100.0, 24.0, 13.0),
                    "16",
                    Action::Count(16),
                    !busy,
                );
                button(
                    g,
                    &mut b,
                    &mut u,
                    Rect(x + 253.0, y + 100.0, 24.0, 13.0),
                    "64",
                    Action::Count(64),
                    !busy,
                );
                let (mut angle, mut chance) = (0.0, u.state.chance);
                let mut result = None;
                if let Some((spin, start)) = &u.spin {
                    chance = spin.chance;
                    let t = (start.elapsed().as_secs_f64() / (spin.ticks as f64 / 20.0))
                        .clamp(0.0, 1.0);
                    angle = (360.0 * 6.0 + spin.angle) * (1.0 - (1.0 - t).powi(4));
                    if t >= 1.0 {
                        result = Some(spin.won);
                    }
                }
                wheel(g, &mut b, x + 144.0, y + 70.0, chance, angle);
                centered(
                    g,
                    &mut b,
                    &format!("{:.2}%", chance * 100.0),
                    x + 144.0,
                    y + 119.0,
                    GOLD,
                );
                let cooldown = u
                    .state
                    .cooldown
                    .saturating_sub(u.state_time.elapsed().as_secs() as u32);
                if let Some(won) = result {
                    if let Some(spin) = &u.last_spin {
                        if spin.nonce != u.played_nonce {
                            u.played_nonce = spin.nonce;
                            g.player_menu_sound(if won {
                                "minecraft:entity.player.levelup"
                            } else {
                                "minecraft:block.anvil.land"
                            });
                        }
                    }
                }
                let cooldown_label = format!("COOLDOWN {cooldown}s");
                let label = if cooldown > 0 {
                    cooldown_label.as_str()
                } else if busy {
                    "ВРАЩАЕТСЯ…"
                } else {
                    match result {
                        Some(true) => "УСПЕХ!",
                        Some(false) => "НЕУДАЧА",
                        None => "УЛУЧШИТЬ",
                    }
                };
                button(
                    g,
                    &mut b,
                    &mut u,
                    Rect(x + 74.0, y + 135.0, 140.0, 19.0),
                    label,
                    Action::Start,
                    !busy && cooldown == 0,
                );
                button(
                    g,
                    &mut b,
                    &mut u,
                    Rect(x + 8.0, y + 135.0, 57.0, 19.0),
                    "Settings",
                    Action::Settings,
                    !busy,
                );
                for row in 0..4 {
                    for col in 0..9 {
                        let slot = if row == 3 { col } else { 9 + row * 9 + col };
                        let iy = if row == 3 {
                            218.0
                        } else {
                            160.0 + row as f32 * 18.0
                        };
                        let r = Rect(x + 63.0 + col as f32 * 18.0, y + iy, 18.0, 18.0);
                        let s = u.state.inventory.get(slot).and_then(Option::as_ref);
                        let shown = s.and_then(|s| {
                            if u.state.selected == Some(slot) {
                                core::unreserved(s, u.state.input.as_ref().map_or(0, |i| i.count))
                            } else {
                                Some(s.clone())
                            }
                        });
                        item_slot(g, &mut b, r, shown.as_ref(), u.state.selected == Some(slot));
                        if !busy {
                            u.hot.push((r, Action::Select(slot)));
                        }
                    }
                }
            }
            Page::Catalog => {
                let query = u.search.clone();
                field(
                    g,
                    &mut b,
                    &mut u,
                    Rect(x + 9.0, y + 23.0, 225.0, 17.0),
                    &query,
                    Edit::Search,
                    true,
                );
                button(
                    g,
                    &mut b,
                    &mut u,
                    Rect(x + 237.0, y + 23.0, 42.0, 17.0),
                    "Назад",
                    Action::Back,
                    true,
                );
                let list = filtered(&u);
                let rows = list.len().div_ceil(14);
                u.row = u.row.min(rows.saturating_sub(9));
                for (r, index) in list.iter().skip(u.row * 14).take(126).enumerate() {
                    let item = u.items[*index].clone();
                    let rect = Rect(
                        x + 18.0 + (r % 14) as f32 * 18.0,
                        y + 44.0 + (r / 14) as f32 * 18.0,
                        18.0,
                        18.0,
                    );
                    item_slot(
                        g,
                        &mut b,
                        rect,
                        Some(&Stack {
                            item: item.id.clone(),
                            count: 1,
                            ..Default::default()
                        }),
                        false,
                    );
                    u.hot.push((rect, Action::Target(item.id.clone())));
                    if rect.has(u.mouse.0, u.mouse.1) {
                        text(g, &mut b, &item.name, x + 9.0, y + 211.0, GOLD);
                    }
                }
                button(
                    g,
                    &mut b,
                    &mut u,
                    Rect(x + 8.0, y + 224.0, 25.0, 13.0),
                    "<",
                    Action::Previous,
                    true,
                );
                button(
                    g,
                    &mut b,
                    &mut u,
                    Rect(x + 255.0, y + 224.0, 25.0, 13.0),
                    ">",
                    Action::Next,
                    true,
                );
                centered(
                    g,
                    &mut b,
                    &format!("{} items · row {} / {}", list.len(), u.row + 1, rows.max(1)),
                    x + 144.0,
                    y + 226.0,
                    DIM,
                );
            }
            Page::Settings => {
                let editable = u.can_edit;
                for (i, (label, _)) in SETTINGS.iter().enumerate() {
                    text(g, &mut b, label, x + 10.0, y + 28.0 + i as f32 * 19.0, FG);
                    let value = setting(&u.draft, i);
                    field(
                        g,
                        &mut b,
                        &mut u,
                        Rect(x + 205.0, y + 23.0 + i as f32 * 19.0, 94.0, 17.0),
                        &value,
                        Edit::Setting(i),
                        editable,
                    );
                }
                button(
                    g,
                    &mut b,
                    &mut u,
                    Rect(x + 10.0, y + 180.0, 140.0, 18.0),
                    "Цены предметов…",
                    Action::Prices,
                    true,
                );
                text(
                    g,
                    &mut b,
                    if editable {
                        "Настройки сервера: можно менять"
                    } else {
                        "Только просмотр: нужны права оператора"
                    },
                    x + 10.0,
                    y + 204.0,
                    DIM,
                );
                button(
                    g,
                    &mut b,
                    &mut u,
                    Rect(x + 10.0, y + 217.0, 140.0, 16.0),
                    "Назад",
                    Action::Back,
                    true,
                );
                button(
                    g,
                    &mut b,
                    &mut u,
                    Rect(x + 160.0, y + 217.0, 140.0, 16.0),
                    "Сохранить",
                    Action::Save,
                    editable,
                );
            }
            Page::Prices => {
                let query = u.search.clone();
                field(
                    g,
                    &mut b,
                    &mut u,
                    Rect(x + 8.0, y + 23.0, 166.0, 17.0),
                    &query,
                    Edit::Search,
                    true,
                );
                let filter = ["Все", "Блоки", "Предметы", "Изменённые"][u.filter as usize];
                button(
                    g,
                    &mut b,
                    &mut u,
                    Rect(x + 177.0, y + 23.0, 70.0, 17.0),
                    filter,
                    Action::Filter,
                    true,
                );
                let sort = ["Имя", "Цена +", "Цена -"][u.sort as usize];
                button(
                    g,
                    &mut b,
                    &mut u,
                    Rect(x + 250.0, y + 23.0, 52.0, 17.0),
                    sort,
                    Action::Sort,
                    true,
                );
                text(g, &mut b, "Предмет", x + 10.0, y + 45.0, DIM);
                text(g, &mut b, "Цена", x + 145.0, y + 45.0, DIM);
                text(g, &mut b, "x", x + 202.0, y + 45.0, DIM);
                text(g, &mut b, "Итог", x + 243.0, y + 45.0, DIM);
                let list = filtered(&u);
                u.row = u.row.min(list.len().saturating_sub(9));
                let editable = u.can_edit;
                for (row, index) in list.iter().skip(u.row).take(9).enumerate() {
                    let i = u.items[*index].clone();
                    let ry = y + 57.0 + row as f32 * 17.0;
                    item_slot(
                        g,
                        &mut b,
                        Rect(x + 8.0, ry, 16.0, 16.0),
                        Some(&Stack {
                            item: i.id.clone(),
                            count: 1,
                            ..Default::default()
                        }),
                        false,
                    );
                    let name: String = i.name.chars().take(17).collect();
                    text(g, &mut b, &name, x + 27.0, ry + 4.0, FG);
                    let value = u
                        .draft
                        .value_overrides
                        .get(&i.id)
                        .map(|v| v.to_string())
                        .unwrap_or_default();
                    field(
                        g,
                        &mut b,
                        &mut u,
                        Rect(x + 135.0, ry, 57.0, 16.0),
                        &value,
                        Edit::Price(*index),
                        editable,
                    );
                    let multiplier = u
                        .draft
                        .value_multipliers
                        .get(&i.id)
                        .map(|v| v.to_string())
                        .unwrap_or_default();
                    field(
                        g,
                        &mut b,
                        &mut u,
                        Rect(x + 195.0, ry, 35.0, 16.0),
                        &multiplier,
                        Edit::Multiplier(*index),
                        editable,
                    );
                    let total = u.draft.price(
                        &i.id,
                        *u.draft.value_overrides.get(&i.id).unwrap_or(&i.base),
                    );
                    text(g, &mut b, &format!("{total:.0}"), x + 233.0, ry + 4.0, DIM);
                    let blocked = u.draft.blocked(&i.id);
                    button(
                        g,
                        &mut b,
                        &mut u,
                        Rect(x + 280.0, ry, 21.0, 16.0),
                        if blocked { "X" } else { "OK" },
                        Action::Blacklist(i.id.clone()),
                        editable && !built_in_blocked(&i.id),
                    );
                }
                button(
                    g,
                    &mut b,
                    &mut u,
                    Rect(x + 8.0, y + 214.0, 27.0, 17.0),
                    "<",
                    Action::Previous,
                    true,
                );
                button(
                    g,
                    &mut b,
                    &mut u,
                    Rect(x + 38.0, y + 214.0, 27.0, 17.0),
                    ">",
                    Action::Next,
                    true,
                );
                button(
                    g,
                    &mut b,
                    &mut u,
                    Rect(x + 75.0, y + 214.0, 105.0, 17.0),
                    "Назад",
                    Action::Back,
                    true,
                );
                button(
                    g,
                    &mut b,
                    &mut u,
                    Rect(x + 190.0, y + 214.0, 110.0, 17.0),
                    "Сохранить",
                    Action::Save,
                    editable,
                );
            }
        }
        if let Some(slot) = u.drag {
            if let Some(s) = u.state.inventory.get(slot).and_then(Option::as_ref) {
                g.draw_item(&mut b, &actual(s), u.mouse.0 - 8.0, u.mouse.1 - 8.0);
            }
        }
        if !u.error.is_empty() {
            let lines = u.error.chars().collect::<Vec<_>>();
            for (i, line) in lines.chunks(65).take(3).enumerate() {
                centered(
                    g,
                    &mut b,
                    &line.iter().collect::<String>(),
                    bw / 2.0,
                    y + 20.0 + i as f32 * 9.0,
                    0xffff7777,
                );
            }
        }
        if u.waiting && u.opened.elapsed().as_secs() > 5 && u.items.is_empty() {
            centered(
                g,
                &mut b,
                "Сервер не ответил: на нём нужен Upgrader",
                bw / 2.0,
                y + h - 12.0,
                0xffff7777,
            );
        }
    });
    let poll = UI.with(|u| {
        let mut u = u.borrow_mut();
        if !u.waiting && u.last_poll.elapsed().as_millis() >= 200 {
            u.last_poll = Instant::now();
            true
        } else {
            false
        }
    });
    if poll {
        send(g, Request::Refresh);
    }
    Some(b)
}
fn act(g: &mut Game<'_>, a: Action) {
    let mut request = None;
    let mut pre_request = None;
    let mut do_close = false;
    UI.with(|ui| {
        let mut u = ui.borrow_mut();
        u.error.clear();
        if matches!(u.focus, Some(Edit::Count)) {
            if let Ok(count) = u.text.parse() {
                pre_request = Some(Request::Target {
                    item: u.state.target.clone(),
                    count,
                });
            }
        }
        edit_commit(&mut u);
        match a {
            Action::Close => do_close = true,
            Action::Back => {
                u.focus = None;
                u.page = if u.page == Page::Prices {
                    Page::Settings
                } else {
                    Page::Main
                };
                u.row = 0;
                u.search.clear()
            }
            Action::Settings => {
                u.page = Page::Settings;
                u.draft = u.config.clone();
                u.error.clear();
                request = Some(Request::Config)
            }
            Action::Catalog => {
                u.page = Page::Catalog;
                u.row = 0;
                u.search.clear();
            }
            Action::Prices => {
                u.page = Page::Prices;
                u.row = 0;
                u.search.clear()
            }
            Action::Select(slot) => {
                request = Some(Request::Select { slot, count: None });
                u.drag = Some(slot)
            }
            Action::SelectPart(slot) => {
                request = Some(Request::SelectHalf { slot });
                u.drag = Some(slot)
            }
            Action::Clear => request = Some(Request::Clear),
            Action::Target(item) => {
                u.page = Page::Main;
                request = Some(Request::Target { item, count: 1 });
            }
            Action::Count(count) => {
                request = Some(Request::Target {
                    item: u.state.target.clone(),
                    count,
                })
            }
            Action::Start => {
                if !spinning(&u) {
                    request = Some(Request::Start)
                }
            }
            Action::Focus(f, s) => {
                u.focus = Some(f);
                u.text = s;
            }
            Action::Filter => {
                u.filter = (u.filter + 1) % 4;
                u.row = 0
            }
            Action::Sort => {
                u.sort = (u.sort + 1) % 3;
                u.row = 0
            }
            Action::Previous => {
                u.row = u
                    .row
                    .saturating_sub(if u.page == Page::Catalog { 9 } else { 9 })
            }
            Action::Next => u.row += if u.page == Page::Catalog { 9 } else { 9 },
            Action::Blacklist(id) => {
                if !u.draft.blacklist.remove(&id) {
                    u.draft.blacklist.insert(id);
                }
            }
            Action::Save => match u.draft.validate() {
                Ok(()) => {
                    request = Some(Request::Save {
                        config: u.draft.clone(),
                    });
                    u.error = "Настройки отправлены серверу".into();
                }
                Err(e) => u.error = e,
            },
        }
    });
    if do_close {
        close(g)
    } else {
        if let Some(r) = pre_request {
            send(g, r)
        }
        if let Some(r) = request {
            send(g, r)
        }
    }
}
pub fn event(g: &mut Game<'_>, event: &WindowEvent) -> bool {
    if !active() {
        if let WindowEvent::MouseInput { button, state, .. } = event {
            return use_item(g, *button, *state == ElementState::Pressed);
        }
        if let WindowEvent::KeyboardInput { event, .. } = event {
            if event.state == ElementState::Pressed
                && event.physical_key == PhysicalKey::Code(KeyCode::KeyU)
                && g.screen == Screen::Playing
            {
                open(g);
                return true;
            }
        }
        return false;
    }
    let mut action = None;
    let mut req = None;
    match event {
        WindowEvent::CursorMoved { position, .. } => {
            let scale = UI.with(|u| HudBatch::new(u.borrow().size).scale);
            g.cursor = (position.x as f32 / scale, position.y as f32 / scale);
            return true;
        }
        WindowEvent::MouseInput { button, state, .. }
            if matches!(button, MouseButton::Left | MouseButton::Right) =>
        {
            if *state == ElementState::Pressed {
                action = UI.with(|u| {
                    let u = u.borrow();
                    let p = g.cursor;
                    u.hot
                        .iter()
                        .rev()
                        .find(|(r, _)| r.has(p.0, p.1))
                        .map(|(_, a)| {
                            if *button == MouseButton::Right {
                                match a {
                                    Action::Select(i) => Action::SelectPart(*i),
                                    _ => a.clone(),
                                }
                            } else {
                                a.clone()
                            }
                        })
                });
            } else {
                UI.with(|u| u.borrow_mut().drag = None);
            }
        }
        WindowEvent::MouseWheel { delta, .. } => UI.with(|u| {
            let mut u = u.borrow_mut();
            let steps = match delta {
                MouseScrollDelta::LineDelta(_, y) => *y,
                MouseScrollDelta::PixelDelta(p) => (p.y / 24.0) as f32,
            };
            if matches!(u.page, Page::Catalog | Page::Prices) {
                if steps > 0.0 {
                    u.row = u.row.saturating_sub(steps.ceil() as usize)
                } else {
                    u.row += (-steps).ceil() as usize
                }
            }
        }),
        WindowEvent::KeyboardInput { event, .. } => {
            if event.state == ElementState::Pressed {
                UI.with(|ui| {
                    let mut u = ui.borrow_mut();
                    if let PhysicalKey::Code(key) = event.physical_key {
                        match key {
                            KeyCode::Escape => {
                                if u.focus.is_some() {
                                    edit_commit(&mut u)
                                } else {
                                    action = Some(if u.page == Page::Main {
                                        Action::Close
                                    } else {
                                        Action::Back
                                    })
                                }
                            }
                            KeyCode::Enter | KeyCode::NumpadEnter => {
                                if matches!(u.focus, Some(Edit::Count)) {
                                    if let Ok(count) = u.text.parse() {
                                        req = Some(Request::Target {
                                            item: u.state.target.clone(),
                                            count,
                                        });
                                    }
                                }
                                edit_commit(&mut u)
                            }
                            KeyCode::Backspace => {
                                u.text.pop();
                                if matches!(u.focus, Some(Edit::Search)) {
                                    u.search = u.text.clone();
                                    u.row = 0
                                }
                            }
                            KeyCode::KeyA
                                if g.modifiers.super_key() || g.modifiers.control_key() =>
                            {
                                u.text.clear()
                            }
                            _ => {
                                if u.focus.is_some() {
                                    if let Some(s) = &event.text {
                                        for c in s.chars().filter(|c| !c.is_control()) {
                                            if u.text.len() < 256 {
                                                u.text.push(c)
                                            }
                                        }
                                        if matches!(u.focus, Some(Edit::Search)) {
                                            u.search = u.text.clone();
                                            u.row = 0
                                        }
                                    }
                                }
                            }
                        }
                    }
                });
            }
        }
        WindowEvent::Ime(winit::event::Ime::Commit(s)) => UI.with(|u| {
            let mut u = u.borrow_mut();
            if u.focus.is_some() {
                u.text.push_str(s);
                if matches!(u.focus, Some(Edit::Search)) {
                    u.search = u.text.clone();
                    u.row = 0
                }
            }
        }),
        WindowEvent::Resized(_)
        | WindowEvent::RedrawRequested
        | WindowEvent::CloseRequested
        | WindowEvent::Focused(_)
        | WindowEvent::ModifiersChanged(_) => return false,
        _ => {}
    }
    if let Some(a) = action {
        act(g, a)
    }
    if let Some(r) = req {
        send(g, r)
    }
    true
}

pub fn settings_open(g: &mut Game<'_>) {
    open(g);
    UI.with(|u| u.borrow_mut().page = Page::Settings);
    send(g, Request::Config);
}

// Expose the same hitboxes to Horizon's built-in UI automation. Clicks still
// enter window_event and use the same action handlers as the player's mouse.
pub fn nodes() -> Option<Vec<horizon_client::widgets::Node>> {
    if !active() {
        return None;
    }
    UI.with(|ui| {
        let u = ui.borrow();
        Some(
            u.hot
                .iter()
                .enumerate()
                .map(|(n, (r, a))| {
                    let id = match a {
                        Action::Close => "close".into(),
                        Action::Settings => "settings".into(),
                        Action::Prices => "prices".into(),
                        Action::Save => "save".into(),
                        Action::Back => "back".into(),
                        Action::Start => "start".into(),
                        Action::Catalog => "catalog".into(),
                        Action::Select(slot) => format!("inventory-{slot}"),
                        Action::Target(id) => format!("target-{id}"),
                        Action::Focus(Edit::Setting(i), _) => format!("setting-{i}"),
                        Action::Focus(Edit::Price(i), _) => format!("price-{i}"),
                        Action::Focus(Edit::Multiplier(i), _) => format!("multiplier-{i}"),
                        Action::Focus(Edit::Search, _) => "search".into(),
                        Action::Focus(Edit::Count, _) => "count".into(),
                        Action::Blacklist(id) => format!("blacklist-{id}"),
                        Action::Filter => "filter".into(),
                        Action::Sort => "sort".into(),
                        Action::Next => "next".into(),
                        Action::Previous => "previous".into(),
                        _ => format!("widget-{n}"),
                    };
                    horizon_client::widgets::Node {
                        id: format!("upgrader-{id}"),
                        kind: horizon_client::widgets::Kind::Button,
                        label: Some(id),
                        rect: [r.0, r.1, r.2, r.3],
                        enabled: true,
                    }
                })
                .collect(),
        )
    })
}
pub fn typed(_g: &mut Game<'_>, s: &str) -> bool {
    UI.with(|ui| {
        let mut u = ui.borrow_mut();
        if !u.open || u.focus.is_none() {
            return false;
        }
        for c in s.replace("\\b", "\u{8}").chars() {
            if c == '\u{8}' {
                u.text.pop();
            } else if !c.is_control() && u.text.len() < 256 {
                u.text.push(c);
            }
        }
        if matches!(u.focus, Some(Edit::Search)) {
            u.search = u.text.clone();
            u.row = 0;
        }
        true
    })
}
