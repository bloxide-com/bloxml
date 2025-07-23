use crate::graph::CodeGenGraph;
use crate::{actor::Actor, blox::state::State};
use std::error::Error;

/// Generate a state implementation for a specific State in the States collection
pub fn generate_inner_states(actor: &Actor, state: &State) -> Result<String, Box<dyn Error>> {
    let state_name = &state.ident;
    let actor_mod = actor.ident.to_lowercase();
    let component_mod = &actor.component.ident;
    let component_ident = &actor.component.ident;
    let message_set = &actor
        .component
        .message_set
        .as_ref()
        .map(|ms| ms.get().ident.clone())
        .unwrap_or(format!("<{component_ident} as Components>::MessageSet"));

    let impl_content = format!(
        r#"use bloxide_tokio::{{components::Components, state_machine::{{StateMachine, State, Transition}}}};
use crate::{actor_mod}::{{component::{component_mod}, messaging::{message_set}}};

/// State implementation for {state_name} state
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct {state_name};

impl State<{component_ident}> for {state_name} {{
    fn handle_message(
        &self,
        state_machine: &mut StateMachine<{component_ident}>,
        message: {message_set},
    ) -> Option<Transition<<{component_ident} as Components>::States, {message_set}>> {{
        None
    }}
}}
"#,
    );
    Ok(impl_content)
}

/// Generate a unified StateEnum implementation that contains all states
pub fn generate_state_enum_impl(actor: &Actor) -> Result<String, Box<dyn Error>> {
    let states = &actor.component.states;
    let actor_mod = actor.ident.to_lowercase();
    let component_ident = &actor.component.ident;
    let enum_name = states.state_enum.get().ident.clone();
    let component_mod = &actor.component.ident;
    let message_set = &actor
        .component
        .message_set
        .as_ref()
        .map(|ms| ms.get().ident.clone())
        .unwrap_or(format!("<{component_ident} as Components>::MessageSet"));

    let mut imports = states.states.iter().fold(String::new(), |acc, state| {
        format!(
            "{acc}use {ident_lowercase}::{ident};\n",
            ident_lowercase = state.ident.to_lowercase(),
            ident = state.ident
        )
    });
    imports.push_str(&format!(
        "use crate::{actor_mod}::{{component::{component_mod}, messaging::{message_set}}};"
    ));

    let variants = states.states.iter().fold(String::new(), |acc, state| {
        format!(
            "{acc}    /// {state_name} state\n    {state_name}({state_name}),\n",
            state_name = state.ident
        )
    });

    let handle_message_arms = states.states.iter().fold(String::new(), |acc, state| {
        format!("{acc}            {enum_name}::{state_name}(state) => state.handle_message(state_machine, message),\n", 
            state_name = state.ident
        )
    });

    let on_entry_arms = states.states.iter().fold(String::new(), |acc, state| {
        format!(
            "{acc}            {enum_name}::{state_name}(state) => state.on_entry(state_machine),\n",
            state_name = state.ident
        )
    });

    let on_exit_arms = states.states.iter().fold(String::new(), |acc, state| {
        format!(
            "{acc}            {enum_name}::{state_name}(state) => state.on_exit(state_machine),\n",
            state_name = state.ident
        )
    });

    let parent_arms = states.states.iter().fold(String::new(), |acc, state| {
        format!(
            "{acc}            {enum_name}::{state_name}(state) => state.parent(),\n",
            state_name = state.ident
        )
    });

    let impl_content = format!(
        r#"use bloxide_tokio::{{components::Components, messaging::MessageSet, state_machine::{{StateMachine, State, StateEnum, Transition}}}};
{imports}

/// Enumeration of all possible states for the actor's state machine
#[derive(Clone, PartialEq, Debug)]
pub enum {enum_name} {{
{variants}}}

impl State<{component_ident}> for {enum_name} {{
    /// Handles incoming messages and returns a transition to a new state if needed
    fn handle_message(
        &self,
        state_machine: &mut StateMachine<{component_ident}>,
        message: {message_set},
    ) -> Option<Transition<<{component_ident} as Components>::States, {message_set}>> {{
        match self {{
{handle_message_arms}
        }}
    }}

    /// Executes actions when entering a state
    fn on_entry(&self, state_machine: &mut StateMachine<{component_ident}>) {{
        match self {{
{on_entry_arms}
        }}
    }}

    /// Executes actions when exiting a state
    fn on_exit(&self, state_machine: &mut StateMachine<{component_ident}>) {{
        match self {{
{on_exit_arms}
        }}
    }}

    /// Returns the parent state in the state machine hierarchy
    fn parent(&self) -> {enum_name} {{
        match self {{
{parent_arms}
        }}
    }}
}}

impl StateEnum for {enum_name} {{
    fn new() -> Self {{
        Self::default()
    }}
}}

impl Default for {enum_name} {{
    fn default() -> Self {{
        {enum_name}::Uninit(Uninit)
    }}
}}
"#,
        message_set = actor.message_set_ident(),
    );

    Ok(impl_content)
}

/// Generate a state implementation with graph-based import resolution
pub fn generate_inner_states_with_graph(
    actor: &Actor,
    state: &State,
    graph: &CodeGenGraph,
) -> Result<String, Box<dyn Error>> {
    let state_name = &state.ident;
    let actor_mod = actor.ident.to_lowercase();
    let component_ident = &actor.component.ident;
    let message_set = &actor
        .component
        .message_set
        .as_ref()
        .map(|ms| ms.get().ident.clone())
        .unwrap_or(format!("<{component_ident} as Components>::MessageSet"));

    // Find the specific state module in the graph to get its imports
    let state_module_path = format!("{actor_mod}::states");
    let imports = if let Some(state_module_idx) = graph
        .graph
        .find_module_by_path_hierarchical(&state_module_path)
    {
        graph
            .get_imports_for_module(state_module_idx)
            .collect::<Vec<_>>()
    } else {
        // Fallback to hardcoded imports if graph lookup fails
        vec![
            "use bloxide_tokio::components::Components;".to_string(),
            "use bloxide_tokio::state_machine::{StateMachine, State, Transition};".to_string(),
            format!("use crate::{actor_mod}::component::{component_ident};"),
            format!("use crate::{actor_mod}::messaging::{message_set};"),
        ]
    };

    let imports_section = if imports.is_empty() {
        String::new()
    } else {
        format!("{}\n\n", imports.join("\n"))
    };

    let impl_content = format!(
        r#"{imports_section}/// State implementation for {state_name} state
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct {state_name};

impl State<{component_ident}> for {state_name} {{
    fn handle_message(
        &self,
        state_machine: &mut StateMachine<{component_ident}>,
        message: {message_set},
    ) -> Option<Transition<<{component_ident} as Components>::States, {message_set}>> {{
        None
    }}
}}
"#,
    );

    Ok(impl_content)
}

/// Generate a unified StateEnum implementation with graph-based import resolution
pub fn generate_state_enum_impl_with_graph(
    actor: &Actor,
    graph: &CodeGenGraph,
) -> Result<String, Box<dyn Error>> {
    let states = &actor.component.states;
    let actor_mod = actor.ident.to_lowercase();
    let component_ident = &actor.component.ident;
    let enum_name = states.state_enum.get().ident.clone();
    let message_set = &actor
        .component
        .message_set
        .as_ref()
        .map(|ms| ms.get().ident.clone())
        .unwrap_or(format!("<{component_ident} as Components>::MessageSet"));

    // Get imports from graph for the states module
    let state_module_path = format!("{actor_mod}::states");
    let mut imports = if let Some(state_module_idx) = graph
        .graph
        .find_module_by_path_hierarchical(&state_module_path)
    {
        graph
            .get_imports_for_module(state_module_idx)
            .collect::<Vec<_>>()
    } else {
        // Fallback to hardcoded imports
        vec![
            "use bloxide_tokio::components::Components;".to_string(),
            "use bloxide_tokio::messaging::MessageSet;".to_string(),
            "use bloxide_tokio::state_machine::{StateMachine, State, StateEnum, Transition};"
                .to_string(),
            format!("use crate::{actor_mod}::component::{component_ident};"),
            format!("use crate::{actor_mod}::messaging::{message_set};"),
        ]
    };

    // Add imports for individual state modules (these are local to the states module)
    for state in &states.states {
        imports.push(format!(
            "use {}::{};",
            state.ident.to_lowercase(),
            state.ident
        ));
    }

    let imports_section = if imports.is_empty() {
        String::new()
    } else {
        format!("{}\n\n", imports.join("\n"))
    };

    // Generate enum variants
    let variants = states.states.iter().fold(String::new(), |acc, state| {
        format!(
            "{acc}    /// {state_name} state\n    {state_name}({state_name}),\n",
            state_name = state.ident
        )
    });

    // Generate match arms for each method
    let handle_message_arms = states.states.iter().fold(String::new(), |acc, state| {
        format!("{acc}            {enum_name}::{state_name}(state) => state.handle_message(state_machine, message),\n", 
            state_name = state.ident
        )
    });

    let on_entry_arms = states.states.iter().fold(String::new(), |acc, state| {
        format!(
            "{acc}            {enum_name}::{state_name}(state) => state.on_entry(state_machine),\n",
            state_name = state.ident
        )
    });

    let on_exit_arms = states.states.iter().fold(String::new(), |acc, state| {
        format!(
            "{acc}            {enum_name}::{state_name}(state) => state.on_exit(state_machine),\n",
            state_name = state.ident
        )
    });

    let parent_arms = states.states.iter().fold(String::new(), |acc, state| {
        format!(
            "{acc}            {enum_name}::{state_name}(state) => state.parent(),\n",
            state_name = state.ident
        )
    });

    let impl_content = format!(
        r#"{imports_section}/// Enumeration of all possible states for the actor's state machine
#[derive(Clone, PartialEq, Debug)]
pub enum {enum_name} {{
{variants}}}

impl State<{component_ident}> for {enum_name} {{
    /// Handles incoming messages and returns a transition to a new state if needed
    fn handle_message(
        &self,
        state_machine: &mut StateMachine<{component_ident}>,
        message: {message_set},
    ) -> Option<Transition<<{component_ident} as Components>::States, {message_set}>> {{
        match self {{
{handle_message_arms}
        }}
    }}

    /// Executes actions when entering a state
    fn on_entry(&self, state_machine: &mut StateMachine<{component_ident}>) {{
        match self {{
{on_entry_arms}
        }}
    }}

    /// Executes actions when exiting a state
    fn on_exit(&self, state_machine: &mut StateMachine<{component_ident}>) {{
        match self {{
{on_exit_arms}
        }}
    }}

    /// Returns the parent state in the state machine hierarchy
    fn parent(&self) -> {enum_name} {{
        match self {{
{parent_arms}
        }}
    }}
}}

impl StateEnum for {enum_name} {{
    fn new() -> Self {{
        Self::default()
    }}
}}

impl Default for {enum_name} {{
    fn default() -> Self {{
        {enum_name}::Uninit(Uninit)
    }}
}}
"#,
    );

    Ok(impl_content)
}

/// Generate state implementations with structural analysis for import detection
pub fn generate_inner_states_with_structural_analysis(
    actor: &Actor,
    state: &State,
    graph: &CodeGenGraph,
) -> Result<String, Box<dyn Error>> {
    let state_name = &state.ident;
    let actor_mod = actor.ident.to_lowercase();
    let component_ident = &actor.component.ident;
    let message_set = &actor
        .component
        .message_set
        .as_ref()
        .map(|ms| ms.get().ident.clone())
        .unwrap_or(format!("<{component_ident} as Components>::MessageSet"));

    // Find the specific state module in the graph to get its imports
    let state_module_path = format!("{actor_mod}::states");
    let imports = if let Some(state_module_idx) = graph
        .graph
        .find_module_by_path_hierarchical(&state_module_path)
    {
        graph
            .get_imports_for_module(state_module_idx)
            .collect::<Vec<_>>()
    } else {
        // Fallback if module not found
        vec![]
    };

    let imports_section = if imports.is_empty() {
        String::new()
    } else {
        format!("{}\n\n", imports.join("\n"))
    };

    let impl_content = format!(
        r#"{imports_section}/// State implementation for {state_name} state
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct {state_name};

impl State<{component_ident}> for {state_name} {{
    fn handle_message(
        &self,
        state_machine: &mut StateMachine<{component_ident}>,
        message: {message_set},
    ) -> Option<Transition<<{component_ident} as Components>::States, {message_set}>> {{
        None
    }}
}}
"#,
    );

    Ok(impl_content)
}

/// Generate a unified StateEnum implementation with structural analysis
pub fn generate_state_enum_impl_with_structural_analysis(
    actor: &Actor,
    graph: &CodeGenGraph,
) -> Result<String, Box<dyn Error>> {
    let states = &actor.component.states;
    let actor_mod = actor.ident.to_lowercase();
    let component_ident = &actor.component.ident;
    let enum_name = states.state_enum.get().ident.clone();
    let message_set = &actor
        .component
        .message_set
        .as_ref()
        .map(|ms| ms.get().ident.clone())
        .unwrap_or(format!("<{component_ident} as Components>::MessageSet"));

    // Get imports from graph for the states module
    let state_module_path = format!("{actor_mod}::states");
    let mut imports = if let Some(state_module_idx) = graph
        .graph
        .find_module_by_path_hierarchical(&state_module_path)
    {
        graph
            .get_imports_for_module(state_module_idx)
            .collect::<Vec<_>>()
    } else {
        // Fallback to hardcoded imports
        vec![
            "use bloxide_tokio::components::Components;".to_string(),
            "use bloxide_tokio::messaging::MessageSet;".to_string(),
            "use bloxide_tokio::state_machine::{StateMachine, State, StateEnum, Transition};"
                .to_string(),
            format!("use crate::{actor_mod}::component::{component_ident};"),
            format!("use crate::{actor_mod}::messaging::{message_set};"),
        ]
    };

    // Add imports for individual state modules (these are local to the states module)
    for state in &states.states {
        imports.push(format!(
            "use {}::{};",
            state.ident.to_lowercase(),
            state.ident
        ));
    }

    let imports_section = if imports.is_empty() {
        String::new()
    } else {
        format!("{}\n\n", imports.join("\n"))
    };

    // Generate enum variants
    let variants = states.states.iter().fold(String::new(), |acc, state| {
        format!(
            "{acc}    /// {state_name} state\n    {state_name}({state_name}),\n",
            state_name = state.ident
        )
    });

    // Generate match arms for each method
    let handle_message_arms = states.states.iter().fold(String::new(), |acc, state| {
        format!("{acc}            {enum_name}::{state_name}(state) => state.handle_message(state_machine, message),\n", 
            state_name = state.ident
        )
    });

    let on_entry_arms = states.states.iter().fold(String::new(), |acc, state| {
        format!(
            "{acc}            {enum_name}::{state_name}(state) => state.on_entry(state_machine),\n",
            state_name = state.ident
        )
    });

    let on_exit_arms = states.states.iter().fold(String::new(), |acc, state| {
        format!(
            "{acc}            {enum_name}::{state_name}(state) => state.on_exit(state_machine),\n",
            state_name = state.ident
        )
    });

    let parent_arms = states.states.iter().fold(String::new(), |acc, state| {
        format!(
            "{acc}            {enum_name}::{state_name}(state) => state.parent(),\n",
            state_name = state.ident
        )
    });

    let impl_content = format!(
        r#"{imports_section}/// Enumeration of all possible states for the actor's state machine
#[derive(Clone, PartialEq, Debug)]
pub enum {enum_name} {{
{variants}}}

impl State<{component_ident}> for {enum_name} {{
    /// Handles incoming messages and returns a transition to a new state if needed
    fn handle_message(
        &self,
        state_machine: &mut StateMachine<{component_ident}>,
        message: {message_set},
    ) -> Option<Transition<<{component_ident} as Components>::States, {message_set}>> {{
        match self {{
{handle_message_arms}
        }}
    }}

    /// Executes actions when entering a state
    fn on_entry(&self, state_machine: &mut StateMachine<{component_ident}>) {{
        match self {{
{on_entry_arms}
        }}
    }}

    /// Executes actions when exiting a state
    fn on_exit(&self, state_machine: &mut StateMachine<{component_ident}>) {{
        match self {{
{on_exit_arms}
        }}
    }}

    /// Returns the parent state in the state machine hierarchy
    fn parent(&self) -> {enum_name} {{
        match self {{
{parent_arms}
        }}
    }}
}}

impl StateEnum for {enum_name} {{
    fn new() -> Self {{
        Self::default()
    }}
}}

impl Default for {enum_name} {{
    fn default() -> Self {{
        {enum_name}::Uninit(Uninit)
    }}
}}
"#,
    );

    Ok(impl_content)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        blox::{
            enums::EnumDef,
            state::{State, StateEnum, States},
        },
        tests::create_test_actor,
    };

    #[test]
    fn test_generate_state_impls() {
        let mut actor = create_test_actor();
        let state = State::from("Create");

        let states = States::new(
            vec![state.clone()],
            StateEnum::new(EnumDef::new("ActorStates", vec![])),
        );
        actor.component.states = states;
        let impl_content =
            generate_inner_states(&actor, &state).expect("Failed to generate state impls");
        let ident = state.ident;
        eprintln!("State impl for {ident}: {impl_content}");

        assert!(impl_content.contains(&format!("pub struct {ident}")));
        assert!(impl_content.contains(&format!("impl State<ActorComponents> for {ident}")));
    }

    #[test]
    fn test_generate_state_enum_impl() {
        let mut actor = create_test_actor();
        let component_ident = &actor.component.ident;
        let state_enum = StateEnum::new(EnumDef::new("ActorStates", vec![]));

        let states = States::new(
            vec![
                State::from("Create"),
                State::new("Update", Some("Create".to_string()), None),
                State::new("Delete", Some("Update".to_string()), None),
            ],
            state_enum,
        );

        actor.component.states = states;

        let impl_content =
            generate_state_enum_impl(&actor).expect("Failed to generate state enum impl");
        eprintln!("State enum impl: {impl_content}");

        assert!(impl_content.contains("pub enum ActorStates"));
        assert!(impl_content.contains(&format!("impl State<{component_ident}> for ActorStates")));

        for state in &actor.component.states.states {
            assert!(impl_content.contains(&format!("    {}({})", state.ident, state.ident)));
        }

        assert!(impl_content.contains("match self {"));
        assert!(impl_content.contains("ActorStates::Create(state) =>"));
    }
}
