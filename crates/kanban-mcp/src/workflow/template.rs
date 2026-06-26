use super::args::RunWorkflowArgs;
use super::parser::{SendHandoffRule, Workflow, WorkflowStep};

pub(super) fn render_template(
    template: &str,
    workflow: &Workflow,
    step: &WorkflowStep,
    rule: &SendHandoffRule,
    args: &RunWorkflowArgs,
    to_agent: &str,
    repo: Option<&str>,
) -> String {
    template
        .replace("{{workflow}}", &workflow.name)
        .replace("{{step}}", &step.name)
        .replace("{{step_agent}}", step.agent.as_deref().unwrap_or(""))
        .replace("{{from_agent}}", &args.from_agent)
        .replace("{{target}}", rule.target.as_deref().unwrap_or(""))
        .replace("{{to_agent}}", to_agent)
        .replace("{{repo}}", repo.unwrap_or(""))
        .replace("{{board}}", args.board.as_deref().unwrap_or(""))
        .replace("{{card}}", args.card.as_deref().unwrap_or(""))
}

pub(super) fn default_body_template() -> &'static str {
    "Workflow {{workflow}} completed step {{step}}. Continue the next action."
}
