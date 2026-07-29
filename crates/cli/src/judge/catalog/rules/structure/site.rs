use super::super::rule;

rule!(
    SITE_SHIP_WRAPPER,
    "structure.site-ship-wrapper",
    "Sites have a ship wrapper",
    "A repository declaring a site exposes one operator ship entrypoint.",
    "Site applications and the ship wrapper seat.",
    Mechanized,
    RELEASE,
    [ADOPTION, SITE_TAG, WEB_TAG]
);
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
