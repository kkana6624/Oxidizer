use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use crate::model::{Lane, PatternType};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PatternStats {
    pub total_notes: u32,
    pub miss_count: u32,
    pub avg_delta_ms: f64,
    pub delta_variance: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LaneStats {
    pub total_notes: u32,
    pub miss_count: u32,
    pub avg_delta_ms: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserProfile {
    pub user_name: String,
    pub pattern_stats: HashMap<PatternType, PatternStats>,
    pub lane_stats: HashMap<Lane, LaneStats>,
    pub last_updated: u64,
}
