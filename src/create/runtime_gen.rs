use crate::blox::actor::Actor;
use std::error::Error;

pub fn generate_runtime(actor: &Actor) -> Result<String, Box<dyn Error>> {
    let message_set_name = actor
        .component
        .message_set
        .as_ref()
        .map(|ms| ms.get().ident.clone())
        .unwrap_or_default();

    let mut select_arms = String::new();
    let iter = actor
        .component
        .message_receivers
        .receivers
        .clone()
        .into_iter()
        .zip(
            actor
                .component
                .message_set
                .clone()
                .unwrap()
                .get()
                .variants
                .clone(),
        );
    for (receiver, variant) in iter {
        select_arms.push_str(&format!(
            r#"                    Some(msg) = self.receivers.{ident}.recv() => {{
                        let current_state = self.state_machine.current_state.clone();
                        self.state_machine.dispatch({message_set_name}::{variant_name}(msg), &current_state);
                    }}
"#,
            ident = receiver.ident,
            variant_name = variant.ident
        ));
    }
    let actor_name = actor.ident.clone();
    let states = &actor.component.states;
    let first_state = &states.states[0];
    let second_state = states.states.get(1).unwrap_or(&states.states[0]);
    let state_enum_name = &states.state_enum.get().ident;

    let content = format!(
        r#"use bloxide_tokio::{{
    components::{{Runnable, *}},
    runtime::*,
    std_exports::*,
}};

use super::{{
    component::{actor_name}Components,
    states::{{
        {first_state_lower}::{first_state},
        {second_state_lower}::{second_state},
        {state_enum_name},
    }},
    messaging::{message_set_name},
}};

impl Runnable<{actor_name}Components> for Blox<{actor_name}Components> {{
    fn run(mut self: Box<Self>) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>> {{
        self.state_machine.init(
            &{state_enum_name}::{first_state}({first_state}),
            &{state_enum_name}::{second_state}({second_state}),
        );

        Box::pin(async move {{
            loop {{
                select! {{
{select_arms}
                }}
            }}
        }})
    }}
}}"#,
        first_state = first_state.ident,
        first_state_lower = first_state.ident.to_lowercase(),
        second_state = second_state.ident,
        second_state_lower = second_state.ident.to_lowercase(),
    );

    Ok(content)
}
