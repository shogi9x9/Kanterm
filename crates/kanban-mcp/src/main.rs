//! kanban-mcp: a stdio MCP server exposing the board to agents.
//!
//! It is a thin shell over `kanban_core::Store`. Per the design review it offers
//! a compact tool surface, addresses cards by their human key (e.g. "KB-12"),
//! never leaks internal ids, and returns compact text rather than JSON so agents
//! spend fewer tokens reading results.

mod error;
mod handlers;
mod instructions;
mod lookup;
mod params;
mod render;

use std::sync::Mutex;

use instructions::SERVER_INSTRUCTIONS;
use kanban_core::Store;
use params::{
    BoardParam, CreateBacklogCardParams, CreateCardsParams, CreateParams, DependencyGraphParams,
    KeyParams, ListParams, ManageBoardsParams, ManageColumnsParams, RecallMemoriesParams,
    RecordMemoryParams, RegisterAgentParams, UpdateParams,
};
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{ServerCapabilities, ServerInfo};
use rmcp::transport::stdio;
use rmcp::{tool, tool_handler, tool_router, ErrorData, ServerHandler, ServiceExt};

struct Kanban {
    store: Mutex<Store>,
    board_id: String,
    db_path: String,
    // Required to initialise the macro-generated router; not read directly.
    #[allow(dead_code)]
    tool_router: ToolRouter<Kanban>,
}

#[tool_router]
impl Kanban {
    fn new(store: Store, board_id: String, db_path: String) -> Self {
        Kanban {
            store: Mutex::new(store),
            board_id,
            db_path,
            tool_router: Self::tool_router(),
        }
    }

    /// Lock the store, recovering from a poisoned mutex so one panicking handler
    /// can't take the whole server down for every subsequent request.
    fn store(&self) -> std::sync::MutexGuard<'_, Store> {
        self.store.lock().unwrap_or_else(|e| e.into_inner())
    }

    #[tool(
        description = "Show a board: each column and the cards in it, plus the list of all boards. Call this first to orient."
    )]
    fn get_board(&self, Parameters(p): Parameters<BoardParam>) -> Result<String, ErrorData> {
        let store = self.store();
        handlers::get_board(&store, &self.board_id, p)
    }

    #[tool(
        description = "List cards as one line each, with optional column/agent_state/query/execution-metadata filters. \
                       Pass queue=executable/review/blocked/claimed/missing_context/dependency_blocked/human \
                       to ask what an agent can work on next. Pass ranked=true to sort matching cards by next-work \
                       suitability and include compact rank reasons."
    )]
    fn list_cards(&self, Parameters(p): Parameters<ListParams>) -> Result<String, ErrorData> {
        let store = self.store();
        handlers::list_cards(&store, &self.board_id, p)
    }

    #[tool(
        description = "Show the task dependency graph for a board, including explicit edges, executable stages, and dependency blockers. Pass active_only=true to hide closed historical edges, or focus=<card key> to show one card's direct neighbours."
    )]
    fn dependency_graph(
        &self,
        Parameters(p): Parameters<DependencyGraphParams>,
    ) -> Result<String, ErrorData> {
        let store = self.store();
        handlers::dependency_graph(&store, &self.board_id, p)
    }

    #[tool(
        description = "Show read-only server status for stale-binary/session diagnosis: version, schema version, DB path, working directory, and default board."
    )]
    fn status(&self) -> Result<String, ErrorData> {
        let store = self.store();
        Ok(handlers::status(&store, &self.board_id, &self.db_path))
    }

    #[tool(
        description = "Get one card in full, including body, priority, assignee, column and agent workflow fields."
    )]
    fn get_card(&self, Parameters(p): Parameters<KeyParams>) -> Result<String, ErrorData> {
        let store = self.store();
        handlers::get_card(&store, &self.board_id, p)
    }

    #[tool(
        description = "Create a new card on a project board. `board` is required: pass an existing project board slug or a new project board name; unknown values create a new project board with the workflow template. This tool never writes to Backlog; use create_card_in_backlog for the protected Backlog inbox. Returns the new card key, destination board slug, and whether the board already existed or was created."
    )]
    fn create_card(&self, Parameters(p): Parameters<CreateParams>) -> Result<String, ErrorData> {
        let mut store = self.store();
        handlers::create_card(&mut store, &self.board_id, p)
    }

    #[tool(
        description = "Create multiple ordered cards on a project board. `board` is required: pass an existing project board slug or a new project board name; unknown values create a new workflow-template project board before importing the plan. This tool never writes to Backlog; use create_card_in_backlog for one-off Backlog inbox capture."
    )]
    fn create_cards(
        &self,
        Parameters(p): Parameters<CreateCardsParams>,
    ) -> Result<String, ErrorData> {
        let mut store = self.store();
        handlers::create_cards(&mut store, &self.board_id, p)
    }

    #[tool(
        description = "Create one card in the protected Backlog inbox. Backlog is opt-in only and has a single Backlog column; use create_card or create_cards with a project board slug/name for project work."
    )]
    fn create_card_in_backlog(
        &self,
        Parameters(p): Parameters<CreateBacklogCardParams>,
    ) -> Result<String, ErrorData> {
        let mut store = self.store();
        handlers::create_card_in_backlog(&mut store, p)
    }

    #[tool(
        description = "Update a card by key. Any field may be set; pass `column` to move it. \
                       Agent execution metadata fields include agent_weight, agent_effort, \
                       suggested_model, expected_tokens, human_intervention, and depends_on. \
                       Use claim / claim_token / release_claim / lease_minutes for agent ownership, \
                       and complete_note to archive the card, mark agent_state=done, clear active handoff \
                       fields, and release any claim."
    )]
    fn update_card(&self, Parameters(p): Parameters<UpdateParams>) -> Result<String, ErrorData> {
        let mut store = self.store();
        handlers::update_card(&mut store, &self.board_id, p)
    }

    #[tool(
        description = "Register this agent process/session and receive an assigned identity \
                       such as `codex#abc123` plus a claim token. Use the assigned identity \
                       as update_card.claim and pass claim_token for claim/release operations. \
                       If you remember a previous assigned identity, pass it as \
                       remembered_identity to rotate the token."
    )]
    fn register_agent(
        &self,
        Parameters(p): Parameters<RegisterAgentParams>,
    ) -> Result<String, ErrorData> {
        let mut store = self.store();
        handlers::register_agent(&mut store, p)
    }

    #[tool(
        description = "Manage a project board's columns. The Backlog board has exactly one \
                       `Backlog` column and rejects column changes. `action` is one of: add (needs `name`), \
                       rename (needs `column` + `new_name`), delete (needs `column` + `to` \
                       destination column for its cards), reorder (needs `column` + `direction` \
                       left|right)."
    )]
    fn manage_columns(
        &self,
        Parameters(p): Parameters<ManageColumnsParams>,
    ) -> Result<String, ErrorData> {
        let mut store = self.store();
        handlers::manage_columns(&mut store, &self.board_id, p)
    }

    #[tool(
        description = "Manage boards. `action` is one of: create (needs `name`; optional \
                       `template` is planning, workflow, or simple and defaults to workflow; \
                       optional `agent_context` stores board-level instructions at creation; \
                       slug and key prefix are derived), archive / unarchive (needs `board` slug; archiving \
                       hides a board but keeps its cards), delete (needs `board` slug; only an \
                       archived board can be deleted, and deletion is permanent), reorder (needs \
                       `board` slug + `direction` up|down), set_context / clear_context \
                       (store board-level agent execution instructions in `agent_context`). \
                       The Backlog board name is reserved; \
                       the Backlog board cannot be archived or deleted. Prefer archive over delete \
                       for finished projects. Use get_board \
                       to see existing board slugs."
    )]
    fn manage_boards(
        &self,
        Parameters(p): Parameters<ManageBoardsParams>,
    ) -> Result<String, ErrorData> {
        let mut store = self.store();
        handlers::manage_boards(&mut store, p)
    }

    #[tool(
        description = "Record a memory: a decision, learning or piece of context that should \
                       survive across sessions. Use it whenever you make a non-obvious design \
                       decision, learn a constraint the hard way, or close a card whose \
                       reasoning future sessions will need. Link it to a card with `card` \
                       (e.g. \"KB-12\"). Returns the memory key (e.g. \"M-3\")."
    )]
    fn record_memory(
        &self,
        Parameters(p): Parameters<RecordMemoryParams>,
    ) -> Result<String, ErrorData> {
        let mut store = self.store();
        handlers::record_memory(&mut store, p)
    }

    #[tool(
        description = "Recall memories (decisions/learnings/context from past sessions), newest \
                       first, one line each. Filter with `query` (substring), `card` (card key) \
                       or `kind`; pass `key` (e.g. \"M-3\") to read one memory in full. Call \
                       this when starting work on a topic to pick up prior decisions."
    )]
    fn recall_memories(
        &self,
        Parameters(p): Parameters<RecallMemoriesParams>,
    ) -> Result<String, ErrorData> {
        let mut store = self.store();
        handlers::recall_memories(&mut store, p)
    }
}

#[tool_handler]
impl ServerHandler for Kanban {
    fn get_info(&self) -> ServerInfo {
        // ServerInfo / Implementation are #[non_exhaustive]; mutate fields rather
        // than construct via literals.
        let mut info = ServerInfo::default();
        info.capabilities = ServerCapabilities::builder().enable_tools().build();
        info.server_info.name = "kanban-mcp".into();
        info.server_info.version = env!("CARGO_PKG_VERSION").into();
        info.instructions = Some(SERVER_INSTRUCTIONS.into());
        info
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let path = match std::env::var_os("KANBAN_DB") {
        Some(p) => std::path::PathBuf::from(p),
        None => Store::default_db_path()?,
    };
    let mut store = Store::open(&path)?;
    let board = store.ensure_default_board()?;
    let handler = Kanban::new(store, board.id, path.display().to_string());

    let service = handler.serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
