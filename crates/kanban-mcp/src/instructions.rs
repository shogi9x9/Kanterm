pub(crate) const SERVER_INSTRUCTIONS: &str = "Local kanban. Call get_board first to orient \
(it also lists all boards; pass `board` to target one). Cards are addressed by their key \
(e.g. KB-12). The default Backlog board has only one `Backlog` column and cannot have \
columns added/renamed/reordered/deleted. Project boards default to the `workflow` \
column template; use explicit `planning` or `simple` when needed. Move a card by calling \
update_card with a `column`. Move a card to another board by setting `move_to_board` \
(board slug) in update_card; when set, column moves apply on the destination board. \
Use create_cards to turn a spec or execution plan into ordered durable cards; include \
alias and depends_on for DAGs such as A -> B/C/D -> E, plus acceptance_criteria, \
next_action, and execution metadata on each item when known. \
For recurring maintenance, create concrete follow-up cards instead of notes: split \
refactor pressure, README/README.ja drift, DESIGN/DESIGN.ja drift, MCP instruction/tool \
description drift, and board agent_context drift into separate cards with next_action and \
acceptance_criteria. \
Use update_card with depends_on to store board-local upstream card keys, and call \
dependency_graph to inspect explicit edges, executable stages, and dependency blockers. \
Use list_cards with queue=executable before claiming work; queue=dependency_blocked explains \
cards waiting on upstream tasks, queue=review surfaces cards \
needing human review, queue=human surfaces all human-gated cards, queue=missing_context \
surfaces cards that lack next_action or acceptance_criteria, and queue=blocked/claimed \
separates unavailable work. \
Agents should call register_agent before claiming cards. It returns an assigned identity \
such as `codex#abc123` plus a claim token; use the assigned identity as update_card.claim \
and pass claim_token for claim/release operations. Inspect get_card agent_metadata before \
claiming: agent_weight, agent_effort, suggested_model, expected_tokens, and human_intervention \
describe suitability, runtime cost, and whether review/decision/execution needs a human. \
Project-board columns can be added/renamed/reordered/deleted with manage_columns, and \
boards created/archived/deleted with manage_boards (create defaults to the `workflow` template; archive finished boards rather than \
deleting them). Use manage_boards action=set_context to store board/project-level agent \
instructions such as verification commands, completion policy, and repo conventions; get_board \
and get_card include this agent_context when set. A memory log lives alongside \
the boards: call recall_memories when starting on a topic to pick up past decisions, and \
record_memory (optionally linked to a card) whenever you make a non-obvious decision or learn a \
lasting constraint.";
