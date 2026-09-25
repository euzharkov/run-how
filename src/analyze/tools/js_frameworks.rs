//! JS frameworks: one slice of the tool knowledge table behind `analyze::tools::summarize`.

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
        "vite" => match sub {
            None | Some("dev") | Some("serve") => {
                s("vite", "Start the Vite development server", Dev, Safe)
            }
            Some("build") => s("vite", "Build the app with Vite", Build, Safe),
            Some("preview") => s("vite", "Preview the production build with Vite", Dev, Safe),
            Some(x) => s("vite", format!("Run vite {x}"), Other, Safe),
        },
        "next" => match sub {
            None | Some("dev") => s("next", "Start the Next.js development server", Dev, Safe),
            Some("build") => s("next", "Build the Next.js app", Build, Safe),
            Some("start") => s("next", "Start the Next.js production server", Dev, Safe),
            Some("lint") => s("next", "Lint the Next.js app", Lint, Safe),
            Some("export") => s("next", "Export the Next.js app as static HTML", Build, Safe),
            Some("telemetry") => s("next", "Configure Next.js telemetry", Other, Safe),
            Some(x) => s("next", format!("Run next {x}"), Other, Safe),
        },
        "nest" => match sub {
            Some("start") if a.has("--watch") || a.has("-w") => {
                s("nest", "Start the NestJS app in watch mode", Dev, Safe)
            }
            Some("start") if a.has("--debug") || a.has("-d") => s(
                "nest",
                "Start the NestJS app with the debugger attached",
                Dev,
                Safe,
            ),
            Some("start") => s("nest", "Start the NestJS app", Dev, Safe),
            Some("build") => s("nest", "Build the NestJS app", Build, Safe),
            Some("generate") | Some("g") => {
                s("nest", "Generate NestJS source files", Generate, Safe)
            }
            Some("new") | Some("n") => s("nest", "Scaffold a new NestJS project", Other, Safe),
            Some("info") | Some("i") => s("nest", "Print NestJS project information", Other, Safe),
            Some(x) => s("nest", format!("Run nest {x}"), Other, Safe),
            None => s("nest", "Run the NestJS CLI", Other, Safe),
        },
        "nuxt" | "nuxi" => match sub {
            None | Some("dev") => s("nuxt", "Start the Nuxt development server", Dev, Safe),
            Some("build") => s("nuxt", "Build the Nuxt app", Build, Safe),
            Some("generate") => s("nuxt", "Generate the static Nuxt site", Build, Safe),
            Some("preview") | Some("start") => {
                s("nuxt", "Preview the Nuxt production build", Dev, Safe)
            }
            Some("typecheck") => s("nuxt", "Check TypeScript types with Nuxt", TypeCheck, Safe),
            Some("prepare") => s("nuxt", "Generate Nuxt type stubs", Generate, Safe),
            Some(x) => s("nuxt", format!("Run nuxt {x}"), Other, Safe),
        },
        "astro" => match sub {
            None | Some("dev") => s("astro", "Start the Astro development server", Dev, Safe),
            Some("build") => s("astro", "Build the Astro site", Build, Safe),
            Some("preview") => s("astro", "Preview the Astro production build", Dev, Safe),
            Some("check") => s(
                "astro",
                "Check the Astro project for errors",
                TypeCheck,
                Safe,
            ),
            Some("sync") => s("astro", "Generate Astro content types", Generate, Safe),
            Some(x) => s("astro", format!("Run astro {x}"), Other, Safe),
        },
        "remix" => match sub {
            Some("dev") | None => s("remix", "Start the Remix development server", Dev, Safe),
            Some("build") => s("remix", "Build the Remix app", Build, Safe),
            Some(x) => s("remix", format!("Run remix {x}"), Other, Safe),
        },
        "react-router" => match sub {
            Some("dev") | None => s(
                "react-router",
                "Start the React Router development server",
                Dev,
                Safe,
            ),
            Some("build") => s("react-router", "Build the React Router app", Build, Safe),
            Some("typegen") => s(
                "react-router",
                "Generate React Router route types",
                Generate,
                Safe,
            ),
            Some(x) => s("react-router", format!("Run react-router {x}"), Other, Safe),
        },
        "svelte-kit" | "vite-plugin-svelte" => s("svelte-kit", "Run SvelteKit", Other, Safe),
        "svelte-check" => s(
            "svelte-check",
            "Check Svelte components and types",
            TypeCheck,
            Safe,
        ),
        "vue-tsc" => {
            if a.has("--noEmit") || a.has("--noemit") {
                s("vue-tsc", "Check Vue and TypeScript types", TypeCheck, Safe)
            } else if a.has_any(&["-b", "--build"]) {
                s(
                    "vue-tsc",
                    "Build TypeScript project references for Vue",
                    Build,
                    Safe,
                )
            } else {
                s("vue-tsc", "Compile Vue and TypeScript sources", Build, Safe)
            }
        }
        "vue-cli-service" => match sub {
            Some("serve") => s("vue-cli", "Start the Vue CLI development server", Dev, Safe),
            Some("build") => s("vue-cli", "Build the Vue app", Build, Safe),
            Some("lint") => s("vue-cli", "Lint the Vue app", Lint, Safe),
            Some("test:unit") => s("vue-cli", "Run Vue unit tests", Test, Safe),
            Some("test:e2e") => s("vue-cli", "Run Vue end-to-end tests", E2e, Safe),
            Some(x) => s("vue-cli", format!("Run vue-cli-service {x}"), Other, Safe),
            None => s(program, format!("Run {program}"), Other, Safe),
        },
        "ng" => match sub {
            Some("serve") | Some("s") => {
                s("angular", "Start the Angular development server", Dev, Safe)
            }
            Some("build") | Some("b") => s("angular", "Build the Angular app", Build, Safe),
            Some("test") | Some("t") => s("angular", "Run Angular unit tests", Test, Safe),
            Some("e2e") | Some("e") => s("angular", "Run Angular end-to-end tests", E2e, Safe),
            Some("lint") => s("angular", "Lint the Angular app", Lint, Safe),
            Some(x) => s("angular", format!("Run ng {x}"), Other, Safe),
            None => s(program, format!("Run {program}"), Other, Safe),
        },
        "react-scripts" | "craco" | "react-app-rewired" => match sub {
            Some("start") => s(
                "react-scripts",
                "Start the Create React App dev server",
                Dev,
                Safe,
            ),
            Some("build") => s("react-scripts", "Build the React app", Build, Safe),
            Some("test") => s("react-scripts", "Run React app tests", Test, Safe),
            Some("eject") => s(
                "react-scripts",
                "Eject from Create React App",
                Other,
                Destructive,
            ),
            Some(x) => s(
                "react-scripts",
                format!("Run react-scripts {x}"),
                Other,
                Safe,
            ),
            None => s(program, format!("Run {program}"), Other, Safe),
        },
        "expo" => match sub {
            None | Some("start") => {
                let on = if a.has("--android") {
                    " for Android"
                } else if a.has("--ios") {
                    " for iOS"
                } else if a.has("--web") {
                    " in the browser"
                } else {
                    ""
                };
                s(
                    "expo",
                    format!("Start the Expo development server{on}"),
                    Dev,
                    Safe,
                )
            }
            Some("run:ios") => s("expo", "Build and run the app on iOS", Dev, Safe),
            Some("run:android") => s("expo", "Build and run the app on Android", Dev, Safe),
            Some("export") => s("expo", "Export the Expo app bundle", Build, Safe),
            Some("prebuild") => s(
                "expo",
                "Generate native iOS and Android projects",
                Generate,
                Safe,
            ),
            Some(x) => s("expo", format!("Run expo {x}"), Other, Safe),
        },
        "eas" => match sub {
            Some("build") if a.has("--local") => {
                s("eas", "Build the app locally with EAS", Build, Safe)
            }
            Some("build") => s("eas", "Build the app with EAS Build", Build, External),
            Some("submit") => s("eas", "Submit the app to the app stores", Publish, External),
            Some("update") => s(
                "eas",
                "Publish an EAS over-the-air update",
                Publish,
                External,
            ),
            Some(x) => s("eas", format!("Run eas {x}"), Other, External),
            None => s(program, format!("Run {program}"), Other, Safe),
        },
        "react-native" => match sub {
            Some("start") | None => s("react-native", "Start the Metro bundler", Dev, Safe),
            Some("run-ios") => s("react-native", "Build and run the app on iOS", Dev, Safe),
            Some("run-android") => s(
                "react-native",
                "Build and run the app on Android",
                Dev,
                Safe,
            ),
            Some(x) => s("react-native", format!("Run react-native {x}"), Other, Safe),
        },
        "electron" => s("electron", "Start the Electron app", Dev, Safe),
        "electron-builder" => s("electron-builder", "Package the Electron app", Build, Safe),
        "electron-forge" => match sub {
            Some("start") => s("electron-forge", "Start the Electron app", Dev, Safe),
            Some("make") => s(
                "electron-forge",
                "Build Electron distributables",
                Build,
                Safe,
            ),
            Some("package") => s("electron-forge", "Package the Electron app", Build, Safe),
            Some("publish") => s(
                "electron-forge",
                "Publish the Electron app",
                Publish,
                External,
            ),
            Some(x) => s(
                "electron-forge",
                format!("Run electron-forge {x}"),
                Other,
                Safe,
            ),
            None => s(program, format!("Run {program}"), Other, Safe),
        },
        "tauri" => match sub {
            Some("dev") => s(
                "tauri",
                "Start the Tauri app in development mode",
                Dev,
                Safe,
            ),
            Some("build") => s("tauri", "Build the Tauri app", Build, Safe),
            Some(x) => s("tauri", format!("Run tauri {x}"), Other, Safe),
            None => s(program, format!("Run {program}"), Other, Safe),
        },
        "storybook" | "start-storybook" | "build-storybook" => match (program, sub) {
            ("build-storybook", _) | (_, Some("build")) => {
                s("storybook", "Build the static Storybook", Build, Safe)
            }
            _ => s("storybook", "Start Storybook", Dev, Safe),
        },
        "docusaurus" => match sub {
            Some("start") => s("docusaurus", "Start the Docusaurus site locally", Dev, Safe),
            Some("build") => s("docusaurus", "Build the Docusaurus site", Docs, Safe),
            Some("deploy") => s("docusaurus", "Deploy the Docusaurus site", Deploy, External),
            Some("serve") => s("docusaurus", "Serve the built Docusaurus site", Dev, Safe),
            Some(x) => s("docusaurus", format!("Run docusaurus {x}"), Other, Safe),
            None => s(program, format!("Run {program}"), Other, Safe),
        },
        "vitepress" => match sub {
            Some("dev") | None => s(
                "vitepress",
                "Start the VitePress docs site locally",
                Dev,
                Safe,
            ),
            Some("build") => s("vitepress", "Build the VitePress docs site", Docs, Safe),
            Some("preview") => s("vitepress", "Preview the built VitePress site", Dev, Safe),
            Some(x) => s("vitepress", format!("Run vitepress {x}"), Other, Safe),
        },
        "typedoc" => s("typedoc", "Generate API docs with TypeDoc", Docs, Safe),
        "mdbook" => match sub {
            Some("serve") => s("mdbook", "Serve the mdBook locally", Dev, Safe),
            Some("build") => s("mdbook", "Build the mdBook", Docs, Safe),
            Some("test") => s("mdbook", "Test the mdBook code samples", Test, Safe),
            Some(x) => s("mdbook", format!("Run mdbook {x}"), Other, Safe),
            None => s(program, format!("Run {program}"), Other, Safe),
        },

        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_sum as sum;
    use super::*;

    #[test]
    fn web_frameworks() {
        assert_eq!(sum("vite").kind, Dev);
        assert_eq!(sum("vite build").text, "Build the app with Vite");
        assert_eq!(sum("next dev").text, "Start the Next.js development server");
        assert_eq!(sum("next build").kind, Build);
        assert_eq!(
            sum("nest start --watch").text,
            "Start the NestJS app in watch mode"
        );
        assert_eq!(sum("nest start").kind, Dev);
        assert_eq!(sum("nest build").kind, Build);
        assert_eq!(sum("nuxt generate").text, "Generate the static Nuxt site");
        assert_eq!(sum("astro check").kind, TypeCheck);
    }

    #[test]
    fn mobile_and_desktop() {
        assert_eq!(
            sum("expo start --ios").text,
            "Start the Expo development server for iOS"
        );
        assert_eq!(sum("eas build --platform ios").risk, External);
        assert_eq!(sum("eas build --local").risk, Safe);
        assert_eq!(sum("eas submit").kind, Publish);
        assert_eq!(sum("react-native run-android").kind, Dev);
        assert_eq!(sum("electron .").text, "Start the Electron app");
    }
}
