use crate::{
    CharacterClient, MapsClient,
    character::{
        MeetsConditionsFor,
        error::MoveError,
    },
    entities::{Map, RawCharacter},
};
use std::sync::Arc;

pub(crate) trait CharacterAction {
    type Result;
    type Error;

    fn execute(&self, handler: &CharacterClient) -> Result<Self::Result, Self::Error>;
    fn can_execute(&self, handler: &CharacterClient) -> Result<(), Self::Error>;
}

pub struct MoveCharacter {
    pub x: i32,
    pub y: i32,
    pub maps: Arc<MapsClient>,
}

impl CharacterAction for MoveCharacter {
    type Result = Map;
    type Error = MoveError;

    fn execute(&self, actionner: &CharacterClient) -> Result<Self::Result, Self::Error> {
        self.can_execute(actionner)?;
        Ok(actionner.handler().request_move(self.x, self.y)?)
    }

    fn can_execute(&self, actionner: &CharacterClient) -> Result<(), Self::Error> {
        let position = actionner.handler().character().position();
        let layer = position.0;
        if position == (layer, self.x, self.y) {
            return Err(MoveError::AlreadyOnMap);
        }
        let Some(map) = self.maps.get((layer, self.x, self.y)) else {
            return Err(MoveError::MapNotFound);
        };
        if map.is_blocked() || !actionner.meets_conditions_for(map.access()) {
            return Err(MoveError::ConditionsNotMet);
        }
        Ok(())
    }
}

//
// pub struct Transition {
//     pub maps: Arc<MapsClient>,
// }
//
// impl CharacterAction for Transition {
//     type Result = Map;
//     type Error = TransitionError;
//
//     fn execute(&self, handler: &CharacterRequestHandler) -> Result<Self::Result, Self::Error> {
//         self.can_execute(handler)?;
//         Ok(handler.request_transition()?)
//     }
//
//     fn can_execute(&self, handler: &CharacterRequestHandler) -> Result<(), Self::Error> {
//         let map = self.maps.get(handler.character().position());
//         let binding = map.unwrap();
//         let Some(transition) = binding.transition() else {
//             return Err(TransitionError::TransitionNotFound);
//         };
//         if !handler.meets_conditions_for(&transition) {
//             return Err(TransitionError::ConditionsNotMet);
//         }
//         Ok(())
//     }
// }
