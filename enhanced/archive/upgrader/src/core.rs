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
