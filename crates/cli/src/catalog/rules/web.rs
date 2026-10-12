use super::rules;

rules! {
    DESIGN_DEPENDENCY => "web.design-dependency",
    DESIGN_PLUGIN_ACTIVE => "web.design-plugin-active",
    VITE_DOES_NOT_OWN_DISPATCH => "web.vite-does-not-own-dispatch",
    VIEWS_MANIFEST_LOADED => "web.views-manifest-loaded",
    VIEWS_MANIFEST_RENDERED => "web.views-manifest-rendered",
    VIEWS_TYPES_DECLARED => "web.views-types-declared",
    BUILD_SCRIPT_PRESENT => "web.build-script-present",
    VIEW_PATH_SEGMENT => "web.view-path-segment",
    VIEW_FILE_KIND => "web.view-file-kind",
    SVELTE_UNDER_COMPONENTS => "web.svelte-under-components",
    CONVENTION_PATH_LOWERCASE => "web.convention-path-lowercase",
    COMPONENT_FILE_KIND => "web.component-file-kind",
    HOOK_FILE_KIND => "web.hook-file-kind",
}
