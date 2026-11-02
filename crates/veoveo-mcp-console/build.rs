//! Embeds the skills under `skills/`, which the server serves at `skill://console/…`.
fn main() {
    veoveo_skills::build::embed_skills("skills").expect("the console's skills");
}
