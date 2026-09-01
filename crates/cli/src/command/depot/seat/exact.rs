pub struct Query<'a> {
    pub source: &'a str,
    pub product: &'a str,
    pub channel: &'a str,
    pub version: &'a str,
    pub derivative: plumb::depot::v2::Kind,
}

pub fn read(query: Query<'_>) -> Result<plumb::depot::v2::Pointer, String> {
    let source = query.source.trim_end_matches('/');
    let key = plumb::depot::v2::exact(
        query.product,
        query.derivative,
        query.channel,
        query.version,
    )?;
    let body = super::pull(&format!("{source}/{key}"))?.ok_or_else(|| {
        format!(
            "depot release {} has no {} pointer",
            query.version,
            query.derivative.label()
        )
    })?;
    let pointer = plumb::depot::v2::Pointer::parse(&body)?;
    let standing = (
        pointer.source.as_str(),
        pointer.release.product.as_str(),
        pointer.release.channel.as_str(),
        pointer.release.version.as_str(),
        pointer.derivative,
    );
    let wanted = (
        source,
        query.product,
        query.channel,
        query.version,
        query.derivative,
    );
    if standing != wanted {
        return Err(format!(
            "exact depot pointer does not name {} {} {} at {source}",
            query.product,
            query.derivative.label(),
            query.version
        ));
    }
    let base = plumb::depot::v2::snapshots(
        &pointer.release,
        pointer.derivative,
        &pointer.snapshot.timestamp,
    )?;
    let body =
        super::pull(&format!("{source}/{base}/{}", plumb::depot::v2::LEAF))?.ok_or_else(|| {
            format!(
                "depot snapshot {} has no manifest",
                pointer.snapshot.timestamp
            )
        })?;
    let manifest = plumb::depot::v2::Manifest::parse(&body)?;
    pointer.bind(&manifest, body.as_bytes())?;
    Ok(pointer)
}
