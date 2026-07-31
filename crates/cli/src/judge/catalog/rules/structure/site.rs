use super::super::rule;

rule!(
    SITE_DEPLOY_LANE,
    "structure.site-deploy-lane",
    "Sites have a deploy lane",
    "A repository declaring a site carries a deploy workflow.",
    "Site applications and workflow names.",
    Mechanized,
    RELEASE,
    [RELEASE_TAG, SITE_TAG, WEB_TAG]
);
