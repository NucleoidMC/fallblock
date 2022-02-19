use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DimensionCodec {
    #[serde(rename = "minecraft:dimension_type")]
    dimension_type: Registry<DimensionType>,
    #[serde(rename = "minecraft:worldgen/biome")]
    worldgen_biome: Registry<Biome>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Registry<T> {
    #[serde(rename = "type")]
    ty: String,
    value: Vec<RegistryEntry<T>>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RegistryEntry<T> {
    name: String,
    id: i32,
    element: T,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DimensionType {
    piglin_safe: bool,
    natural: bool,
    ambient_light: f32,
    fixed_time: Option<i64>,
    infiniburn: String,
    respawn_anchor_works: bool,
    has_skylight: bool,
    bed_works: bool,
    effects: String,
    has_raids: bool,
    min_y: i32,
    height: i32,
    logical_height: i32,
    coordinate_scale: f32,
    ultrawarm: bool,
    has_ceiling: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Biome {
    precipitation: String,
    depth: f32,
    temperature: f32,
    scale: f32,
    downfall: f32,
    category: String,
    temperature_modifier: Option<String>,
    effects: Effects,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Effects {
    sky_color: i32,
    water_fog_color: i32,
    fog_color: i32,
    water_color: i32,
    foliage_color: Option<i32>,
    grass_color: Option<i32>,
    grass_color_modifier: Option<String>,
    music: Option<()>, // I CBA
    ambient_sound: Option<String>,
    additions_sound: Option<()>,
    mood_sound: Option<()>,
}
