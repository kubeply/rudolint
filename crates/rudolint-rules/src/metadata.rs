use rudolint_diagnostics::{Finding, Severity};
use rudolint_dockerfile::Instruction;

macro_rules! rule {
    ($name:ident, $code:literal, $severity:expr, $summary:literal, $body:expr) => {
        pub(crate) struct $name;
        impl crate::Rule for $name {
            fn info(&self) -> crate::RuleInfo {
                crate::RuleInfo {
                    code: $code,
                    severity: $severity,
                    summary: $summary,
                    status: crate::RuleStatus::Implemented,
                }
            }

            fn check(
                &self,
                document: &rudolint_dockerfile::Dockerfile,
            ) -> Vec<rudolint_diagnostics::Finding> {
                $body(document)
            }
        }
    };
}

pub(crate) use rule;

pub(crate) fn diagnostic(
    code: &'static str,
    severity: Severity,
    message: impl Into<String>,
    instruction: &Instruction,
) -> Finding {
    Finding::new(code, severity, message, instruction.line, 1)
}
