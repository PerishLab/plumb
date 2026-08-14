use super::{Rule, rule};

rule!(
    DESIGN_DEPENDENCY,
    "web.design-dependency",
    "Web uses the design system",
    "A Svelte/Vite app depends on the workshop design package.",
    "The web package dependency maps.",
    Mechanized,
    WEB,
    [DEPENDENCY, WEB_TAG]
);
rule!(
    DESIGN_PLUGIN_DEPENDENCY,
    "web.design-plugin-dependency",
    "Web carries the design plugin",
    "A Svelte/Vite app carries the design plugin through the design package.",
    "The web package dependency maps.",
    Mechanized,
    WEB,
    [DEPENDENCY, WEB_TAG]
);
rule!(
    DESIGN_PLUGIN_ACTIVE,
    "web.design-plugin-active",
    "The design plugin is active",
    "The Vite configuration activates the workshop design plugin.",
    "Recognized Vite configuration files.",
    Mechanized,
    WEB,
    [CONFIGURATION, WEB_TAG]
);
rule!(
    VITE_DOES_NOT_OWN_DISPATCH,
    "web.vite-does-not-own-dispatch",
    "Vite does not own sidecar dispatch",
    "Vite receives dispatch through the design runtime rather than reading sidecar environment.",
    "Recognized Vite configuration files.",
    Mechanized,
    WEB,
    [DISPATCH, WEB_TAG]
);
rule!(
    VIEWS_MANIFEST_LOADED,
    "web.views-manifest-loaded",
    "Web loads the views manifest",
    "The web source imports the virtual views manifest.",
    "TypeScript source under apps/web/src.",
    Mechanized,
    WEB,
    [WEB_TAG]
);
rule!(
    VIEWS_MANIFEST_RENDERED,
    "web.views-manifest-rendered",
    "Web renders the views manifest",
    "The web source renders its virtual views through the design system.",
    "TypeScript source under apps/web/src.",
    Mechanized,
    WEB,
    [WEB_TAG]
);
rule!(
    VIEWS_TYPES_DECLARED,
    "web.views-types-declared",
    "Web declares the views module type",
    "The web sources declare the virtual views module so its catalog type is checked.",
    "Ambient declarations under apps/web/src.",
    Mechanized,
    WEB,
    [CONFIGURATION, WEB_TAG]
);
rule!(
    BUILD_SCRIPT_PRESENT,
    "web.build-script-present",
    "Web exposes a build script",
    "The web package declares a build script.",
    "The scripts map in apps/web/package.json.",
    Mechanized,
    WEB,
    [WEB_TAG]
);
rule!(
    GUARD_BUILDS_WEB,
    "web.guard-builds-web",
    "Guard builds the web app",
    "The repository guard builds the named web package.",
    "The web package name and canonical guard workflow source.",
    Mechanized,
    WEB,
    [ADOPTION, WEB_TAG]
);
rule!(
    VIEW_PATH_SEGMENT,
    "web.view-path-segment",
    "View paths are route segments",
    "View directories and route stems use lowercase route-segment grammar.",
    "Names below apps/web/src/views.",
    Mechanized,
    WEB,
    [WEB_TAG]
);
rule!(
    VIEW_FILE_KIND,
    "web.view-file-kind",
    "Views contain route TSX files",
    "The views tree contains only route .svelte files.",
    "Files below apps/web/src/views.",
    Mechanized,
    WEB,
    [WEB_TAG]
);
rule!(
    SVELTE_UNDER_COMPONENTS,
    "web.svelte-under-components",
    "Library TSX lives under components",
    "Reusable web library TSX lives below lib/components.",
    "Direct files below apps/web/src/lib.",
    Mechanized,
    WEB,
    [WEB_TAG]
);
rule!(
    CONVENTION_PATH_LOWERCASE,
    "web.convention-path-lowercase",
    "Convention paths are lowercase",
    "Component and hook convention paths use lowercase names.",
    "Names below web component and hook directories.",
    Mechanized,
    WEB,
    [WEB_TAG]
);
rule!(
    COMPONENT_FILE_KIND,
    "web.component-file-kind",
    "Components are TSX files",
    "The web component tree contains only lowercase directories and .svelte files.",
    "Files below apps/web/src/lib/components.",
    Mechanized,
    WEB,
    [WEB_TAG]
);
rule!(
    HOOK_FILE_KIND,
    "web.hook-file-kind",
    "Hooks are lowercase .ts files",
    "The web hook tree contains lowercase .ts files. The directory identifies a hook, so the file name does not carry a use- prefix; that spelling is two words and the single-word law refuses it. Name the file for the subject and let it export the hooks: health.ts exports useHealth.",
    "Files below apps/web/src/lib/hooks.",
    Mechanized,
    WEB,
    [WEB_TAG]
);

pub fn all() -> Vec<&'static Rule> {
    vec![
        &BUILD_SCRIPT_PRESENT,
        &VIEWS_TYPES_DECLARED,
        &COMPONENT_FILE_KIND,
        &CONVENTION_PATH_LOWERCASE,
        &DESIGN_PLUGIN_ACTIVE,
        &DESIGN_PLUGIN_DEPENDENCY,
        &GUARD_BUILDS_WEB,
        &HOOK_FILE_KIND,
        &DESIGN_DEPENDENCY,
        &SVELTE_UNDER_COMPONENTS,
        &VIEW_FILE_KIND,
        &VIEW_PATH_SEGMENT,
        &VIEWS_MANIFEST_LOADED,
        &VIEWS_MANIFEST_RENDERED,
        &VITE_DOES_NOT_OWN_DISPATCH,
    ]
}
