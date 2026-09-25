//! JS tests: one slice of the tool knowledge table behind `analyze::tools::summarize`.

#[allow(unused_imports)]
use super::Kind::*;
#[allow(unused_imports)]
use super::{clean_path, compose, is_build_output, list};
use super::{s, Args, Summary};
#[allow(unused_imports)]
use crate::model::Risk::*;

#[allow(unused_variables, clippy::needless_return)]
pub(super) fn summarize(
    program: &str,
    args: &[String],
    a: &Args,
    sub: Option<&str>,
) -> Option<Summary> {
    match program {
        "vitest" => {
            let cov = a.has("--coverage");
            match sub {
                Some("run") | None | Some("watch") | Some("dev") => {
                    let watch = sub == Some("watch")
                        || sub == Some("dev")
                        || a.has("--watch")
                        || a.has("-w");
                    let t = if cov {
                        "Run Vitest tests with coverage"
                    } else if watch {
                        "Run Vitest tests in watch mode"
                    } else {
                        "Run Vitest tests"
                    };
                    s("vitest", t, Test, Safe)
                }
                Some("bench") => s("vitest", "Run Vitest benchmarks", Bench, Safe),
                Some("ui") => s("vitest", "Open the Vitest UI", Test, Safe),
                Some("typecheck") => s("vitest", "Run Vitest type checks", TypeCheck, Safe),
                Some("list") => s("vitest", "List Vitest tests", Test, Safe),
                Some(x) => s("vitest", format!("Run Vitest tests in {}", x), Test, Safe),
            }
        }
        "jest" => {
            let t = if a.has("--coverage") {
                "Run Jest tests with coverage"
            } else if a.has("--watch") || a.has("--watchAll") {
                "Run Jest tests in watch mode"
            } else {
                "Run Jest tests"
            };
            s("jest", t, Test, Safe)
        }
        "mocha" => s("mocha", "Run Mocha tests", Test, Safe),
        "ava" => s("ava", "Run AVA tests", Test, Safe),
        "tap" | "node-tap" => s("tap", "Run node-tap tests", Test, Safe),
        "uvu" => s("uvu", "Run uvu tests", Test, Safe),
        "karma" => s("karma", "Run Karma browser tests", Test, Safe),
        "jasmine" => s("jasmine", "Run Jasmine tests", Test, Safe),
        "playwright" => match sub {
            Some("test") | None => {
                if a.has("--ui") {
                    s("playwright", "Open the Playwright test UI", E2e, Safe)
                } else {
                    s("playwright", "Run Playwright end-to-end tests", E2e, Safe)
                }
            }
            Some("install") => s("playwright", "Download Playwright browsers", Install, Safe),
            Some("codegen") => s(
                "playwright",
                "Record a Playwright test interactively",
                E2e,
                Safe,
            ),
            Some("show-report") => s("playwright", "Open the Playwright HTML report", E2e, Safe),
            Some(x) => s("playwright", format!("Run playwright {x}"), E2e, Safe),
        },
        "cypress" => match sub {
            Some("open") => s("cypress", "Open the Cypress test runner", E2e, Safe),
            Some("run") | None => s("cypress", "Run Cypress end-to-end tests", E2e, Safe),
            Some(x) => s("cypress", format!("Run cypress {x}"), E2e, Safe),
        },
        "wdio" | "webdriverio" => s("wdio", "Run WebdriverIO tests", E2e, Safe),
        "detox" => s("detox", "Run Detox end-to-end tests", E2e, Safe),
        "c8" | "nyc" => s(program, "Run tests with coverage", Test, Safe),
        "codecov" => s("codecov", "Upload coverage to Codecov", Publish, External),
        "lighthouse" => s("lighthouse", "Audit a page with Lighthouse", Test, Safe),
        "tsd" => s("tsd", "Test TypeScript type definitions", TypeCheck, Safe),
        "expect-type" | "attw" | "are-the-types-wrong" => {
            s("attw", "Check published type definitions", TypeCheck, Safe)
        }
        "publint" => s("publint", "Lint the package for publishing", Lint, Safe),
        "size-limit" => s("size-limit", "Check the bundle size", Test, Safe),

        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_sum as sum;
    use super::*;

    #[test]
    fn test_runners() {
        assert_eq!(
            sum("vitest run --coverage").text,
            "Run Vitest tests with coverage"
        );
        assert_eq!(sum("vitest --watch").text, "Run Vitest tests in watch mode");
        assert_eq!(sum("vitest bench").kind, Bench);
        assert_eq!(sum("jest --coverage").text, "Run Jest tests with coverage");
        assert_eq!(sum("mocha").kind, Test);
    }

    #[test]
    fn end_to_end_runners() {
        assert_eq!(
            sum("playwright test --ui").text,
            "Open the Playwright test UI"
        );
        assert_eq!(sum("playwright install").kind, Install);
        assert_eq!(sum("cypress open").kind, E2e);
        assert_eq!(sum("cypress run").text, "Run Cypress end-to-end tests");
        assert_eq!(sum("codecov").risk, External);
        assert_eq!(sum("size-limit").risk, Safe);
    }
}
