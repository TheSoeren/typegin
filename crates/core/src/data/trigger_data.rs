use serde::Deserialize;

use crate::{DataCondition, DataEffect, TriggerId, WorldDataError};

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct TriggerData {
    #[serde(rename = "key")]
    pub id: TriggerId,
    #[serde(default)]
    pub condition: Vec<DataCondition>,
    #[serde(default)]
    pub effect: Vec<DataEffect>,
}
impl TriggerData {
    pub(crate) fn validate_references(
        &self,
        data: &super::WorldData,
    ) -> Result<(), WorldDataError> {
        for condition in &self.condition {
            condition.validate_references(data)?;
        }

        for effect in &self.effect {
            effect.validate_references(data)?;
        }

        Ok(())
    }
}
