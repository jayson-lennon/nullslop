#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    clippy::indexing_slicing,
    reason = "test code"
)]

use std::path::PathBuf;
use std::sync::Arc;

use kameo::prelude::Spawn;

use crate::common::actor_deps::ActorDeps;
use crate::common::bus::test_harness::{TestHarness, await_recorded};
use crate::common::services::test_services::TestServices;
use crate::feat::browser::BrowserBinary;
use crate::feat::provider_infra::ProvidersConfig;
use crate::init::env_init_actor::EnvironmentLoaded;

use super::binary_resolver::{BinaryFamily, BinaryLocator};
use super::{BrowserBinaryScanActor, BrowserBinaryScanActorDeps, BrowserBinaryVerified};

/// A fake filesystem that reports Chrome present, Chromium present, both, or neither.
#[derive(Default)]
struct FakeFs {
    chrome: Option<PathBuf>,
    chromium: Option<PathBuf>,
}

impl BinaryLocator for FakeFs {
    fn candidates(&self, family: BinaryFamily) -> Vec<PathBuf> {
        match family {
            BinaryFamily::Chrome => self.chrome.iter().cloned().collect(),
            BinaryFamily::Chromium => self.chromium.iter().cloned().collect(),
            BinaryFamily::Bundled => Vec::new(),
        }
    }
    fn exists(&self, _path: &std::path::Path) -> bool {
        // The fake's candidates list IS the set of existing paths.
        true
    }
}

async fn harness_with_locator(
    config: BrowserBinary,
    locator: Arc<dyn BinaryLocator + Send + Sync>,
) -> (TestHarness, kameo::actor::ActorRef<BrowserBinaryScanActor>) {
    let harness = TestHarness::new().await;
    let mut services = TestServices::builder().build();
    services.bus = harness.bus();
    let deps = ActorDeps { services };
    let actor = BrowserBinaryScanActor::spawn(BrowserBinaryScanActorDeps {
        deps,
        config,
        locator,
    });
    actor.wait_for_startup().await;
    (harness, actor)
}

#[rstest::rstest]
#[tokio::test]
async fn environment_loaded_with_present_binary_emits_verified() {
    // Given an actor configured for Auto where Chrome exists.
    let locator = Arc::new(FakeFs {
        chrome: Some(PathBuf::from("/usr/bin/google-chrome")),
        chromium: None,
    }) as Arc<dyn BinaryLocator + Send + Sync>;
    let (harness, _actor) = harness_with_locator(BrowserBinary::Auto, locator).await;

    // Subscribe to the verified event BEFORE publishing, then publish.
    let recorder = harness.spawn_recorder::<BrowserBinaryVerified>().await;
    harness
        .publish(EnvironmentLoaded {
            config: ProvidersConfig {
                providers: std::collections::BTreeMap::new(),
                aliases: vec![],
                default_provider: None,
            },
        })
        .await;

    // When waiting for the event.
    let messages = await_recorded(&recorder, 1, std::time::Duration::from_secs(2)).await;

    // Then a BrowserBinaryVerified event was emitted for the Chrome family.
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].family, BinaryFamily::Chrome);
    assert_eq!(
        messages[0].path.as_deref(),
        Some(std::path::Path::new("/usr/bin/google-chrome"))
    );
}

#[rstest::rstest]
#[tokio::test]
async fn environment_loaded_with_no_binary_falls_back_to_bundled() {
    // Given an actor configured for Chrome where neither binary exists.
    let locator = Arc::new(FakeFs::default()) as Arc<dyn BinaryLocator + Send + Sync>;
    let (harness, _actor) = harness_with_locator(BrowserBinary::Chrome, locator).await;

    // Subscribe to the verified event (resolution always yields Verified now).
    let recorder = harness.spawn_recorder::<BrowserBinaryVerified>().await;
    harness
        .publish(EnvironmentLoaded {
            config: ProvidersConfig {
                providers: std::collections::BTreeMap::new(),
                aliases: vec![],
                default_provider: None,
            },
        })
        .await;

    // When waiting for the event.
    let messages = await_recorded(&recorder, 1, std::time::Duration::from_secs(2)).await;

    // Then a Verified event was emitted for the Bundled family with a fallback note.
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].family, BinaryFamily::Bundled);
    assert!(messages[0].path.is_none());
    assert!(messages[0].fallback_note.is_some());
}

#[rstest::rstest]
#[tokio::test]
async fn auto_falls_back_to_chromium_when_chrome_absent() {
    // Given an actor configured for Auto where only Chromium exists.
    let locator = Arc::new(FakeFs {
        chrome: None,
        chromium: Some(PathBuf::from("/usr/bin/chromium")),
    }) as Arc<dyn BinaryLocator + Send + Sync>;
    let (harness, _actor) = harness_with_locator(BrowserBinary::Auto, locator).await;

    // Subscribe then publish.
    let recorder = harness.spawn_recorder::<BrowserBinaryVerified>().await;
    harness
        .publish(EnvironmentLoaded {
            config: ProvidersConfig {
                providers: std::collections::BTreeMap::new(),
                aliases: vec![],
                default_provider: None,
            },
        })
        .await;

    // When waiting for the event.
    let messages = await_recorded(&recorder, 1, std::time::Duration::from_secs(2)).await;
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].family, BinaryFamily::Chromium);
}

#[rstest::rstest]
#[tokio::test]
async fn environment_loaded_publishes_service_status_update_for_web_fetch() {
    // Given an actor configured for Auto where Chrome exists.
    let locator = Arc::new(FakeFs {
        chrome: Some(PathBuf::from("/usr/bin/google-chrome")),
        chromium: None,
    }) as Arc<dyn BinaryLocator + Send + Sync>;
    let (harness, _actor) = harness_with_locator(BrowserBinary::Auto, locator).await;

    // Subscribe to the dashboard projection event, then publish.
    let status_recorder = harness
        .spawn_recorder::<crate::feat::dashboard::ServiceStatusUpdate>()
        .await;
    harness
        .publish(EnvironmentLoaded {
            config: ProvidersConfig {
                providers: std::collections::BTreeMap::new(),
                aliases: vec![],
                default_provider: None,
            },
        })
        .await;

    // When waiting for the event.
    let messages = await_recorded(&status_recorder, 1, std::time::Duration::from_secs(2)).await;

    // Then a ServiceStatusUpdate for the web-fetch row was published.
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].name, "web-fetch");
    // And it carries the display label in the status message. The FakeFs
    // locator cannot detect versions, so the label falls back to CHROME_MAJOR.
    {
        let expected = format!(
            "Chrome {} (version undetected) — /usr/bin/google-chrome",
            jinn_web_fetch::stealth::CHROME_MAJOR
        );
        assert_eq!(
            messages[0].status_message.as_deref(),
            Some(expected.as_str())
        );
    }
    // And it does not touch lifecycle or description (owned by the actor events).
    assert!(messages[0].lifecycle.is_none());
    assert!(messages[0].description.is_none());
}

#[rstest::rstest]
#[case::chrome_with_path_and_version(
    BrowserBinaryVerified {
        family: BinaryFamily::Chrome,
        path: Some(PathBuf::from("/usr/bin/google-chrome")),
        version_major: Some("138".to_owned()),
        fallback_note: None,
    },
    "Chrome 138 — /usr/bin/google-chrome",
)]
#[case::bundled_with_fallback_note(
    BrowserBinaryVerified {
        family: BinaryFamily::Bundled,
        path: None,
        version_major: None,
        fallback_note: Some("No system Chrome/Chromium — using bundled".to_owned()),
    },
    "No system Chrome/Chromium — using bundled: Chromium (bundled, version undetected)",
)]
#[case::chromium_version_undetected_falls_back_to_chrome_major(
    BrowserBinaryVerified {
        family: BinaryFamily::Chromium,
        path: Some(PathBuf::from("/usr/bin/chromium")),
        version_major: None,
        fallback_note: None,
    },
    {
        let expected = format!(
            "Chromium {} (version undetected) — /usr/bin/chromium",
            jinn_web_fetch::stealth::CHROME_MAJOR
        );
        expected
    },
)]
fn display_label_formats_family_version_path_and_note(
    #[case] verified: BrowserBinaryVerified,
    #[case] expected: String,
) {
    // Given a verified browser binary resolution.

    // When building the dashboard display label.
    let label = verified.display_label();

    // Then the label matches the expected format.
    assert_eq!(label, expected);
}

#[rstest::rstest]
fn display_label_with_no_path_omits_path_suffix() {
    // Given a Chrome resolution without a path.
    let verified = BrowserBinaryVerified {
        family: BinaryFamily::Chrome,
        path: None,
        version_major: Some("139".to_owned()),
        fallback_note: None,
    };

    // When building the display label.
    let label = verified.display_label();

    // Then the label has no path suffix.
    assert_eq!(label, "Chrome 139");
}
