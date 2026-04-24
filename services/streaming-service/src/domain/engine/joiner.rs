use crate::models::topology::JoinDefinition;

pub fn simulate_join_output(join_definition: Option<&JoinDefinition>, source_count: usize) -> i32 {
    match join_definition {
        Some(definition) => {
            let base = 24 + source_count as i32 * 11;
            let window_bonus = (definition.window_seconds / 60).max(1);
            base + window_bonus
        }
        None => 0,
    }
}
