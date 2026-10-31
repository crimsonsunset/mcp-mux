use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use futures::FutureExt;
use uuid::Uuid;

use super::super::super::MetaToolError;
use super::super::approval_types::{ApprovalPayload, ApprovalRequest};
use super::{ApprovalBroker, ApprovalDecision, ApprovalPublisher};
use crate::services::ANONYMOUS_CLIENT_ID;

fn make_payload() -> ApprovalPayload {
    payload_for(None)
}

fn payload_for(space_id: Option<Uuid>) -> ApprovalPayload {
    payload_with(None, space_id)
}

fn payload_with(action: Option<&str>, space_id: Option<Uuid>) -> ApprovalPayload {
    ApprovalPayload {
        tool_name: "mcpmux_pin_this_session".into(),
        summary: "test".into(),
        space_name: None,
        space_id,
        diff: None,
        action: action.map(str::to_string),
        allow_always: false,
        raw_args: serde_json::json!({}),
        affects_other_clients: false,
    }
}

/// A publisher that answers every prompt with `decision` and counts the prompts.
fn answering(
    broker: &Arc<ApprovalBroker>,
    decision: ApprovalDecision,
    prompts: Arc<AtomicUsize>,
) -> ApprovalPublisher {
    let broker = broker.clone();
    Arc::new(move |req| {
        let b = broker.clone();
        let prompts = prompts.clone();
        async move {
            prompts.fetch_add(1, Ordering::SeqCst);
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_millis(10)).await;
                b.respond(
                    &req.request_id,
                    &req.client_id,
                    &req.payload.tool_name,
                    decision,
                );
            });
            true
        }
        .boxed()
    })
}

#[tokio::test]
async fn no_publisher_returns_no_desktop_error() {
    let broker = ApprovalBroker::new();
    let err = broker
        .request_approval(
            &Uuid::new_v4().to_string(),
            "mcpmux_pin_this_session",
            make_payload(),
        )
        .await
        .unwrap_err();
    assert!(matches!(err, MetaToolError::ApprovalRequiredNoDesktop));
}

#[tokio::test]
async fn always_allow_short_circuits() {
    let broker = ApprovalBroker::new();
    let client_id = Uuid::new_v4().to_string();
    broker.insert_always_allow(&client_id, "mcpmux_pin_this_session", None, None);
    let d = broker
        .request_approval(&client_id, "mcpmux_pin_this_session", make_payload())
        .await
        .unwrap();
    assert_eq!(d, ApprovalDecision::AllowOnce);
}

#[tokio::test]
async fn url_client_id_works() {
    // Regression for the bug where DCR-registered clients (which use
    // a client_metadata URL as their client_id) couldn't get past the
    // approval flow because we tried to parse the URL as a UUID.
    let broker = ApprovalBroker::new();
    let url_client_id = "https://claude.ai/oauth/claude-code-client-metadata";
    broker.insert_always_allow(url_client_id, "mcpmux_pin_this_session", None, None);
    let d = broker
        .request_approval(url_client_id, "mcpmux_pin_this_session", make_payload())
        .await
        .unwrap();
    assert_eq!(d, ApprovalDecision::AllowOnce);
}

#[tokio::test]
async fn publisher_allow_resolves() {
    let broker = Arc::new(ApprovalBroker::new().with_timeout(Duration::from_millis(500)));
    let broker_clone = broker.clone();
    let client_id = Uuid::new_v4().to_string();

    // Publisher responds asynchronously with Allow.
    let publisher: ApprovalPublisher = Arc::new(move |req| {
        let b = broker_clone.clone();
        async move {
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_millis(10)).await;
                b.respond(
                    &req.request_id,
                    &req.client_id,
                    &req.payload.tool_name,
                    ApprovalDecision::AllowOnce,
                );
            });
            true
        }
        .boxed()
    });
    broker.set_publisher(publisher).await;

    let decision = broker
        .request_approval(&client_id, "mcpmux_pin_this_session", make_payload())
        .await
        .unwrap();
    assert_eq!(decision, ApprovalDecision::AllowOnce);
}

#[tokio::test]
async fn publisher_deny_returns_denied_error() {
    let broker = Arc::new(ApprovalBroker::new().with_timeout(Duration::from_millis(500)));
    let broker_clone = broker.clone();
    let client_id = Uuid::new_v4().to_string();

    let publisher: ApprovalPublisher = Arc::new(move |req| {
        let b = broker_clone.clone();
        async move {
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_millis(10)).await;
                b.respond(
                    &req.request_id,
                    &req.client_id,
                    &req.payload.tool_name,
                    ApprovalDecision::Deny,
                );
            });
            true
        }
        .boxed()
    });
    broker.set_publisher(publisher).await;

    let err = broker
        .request_approval(&client_id, "mcpmux_pin_this_session", make_payload())
        .await
        .unwrap_err();
    assert!(matches!(err, MetaToolError::ApprovalDenied));
}

#[tokio::test]
async fn publisher_timeout() {
    let broker = Arc::new(ApprovalBroker::new().with_timeout(Duration::from_millis(50)));
    // Publisher accepts delivery but never responds.
    let publisher: ApprovalPublisher = Arc::new(move |_req| async move { true }.boxed());
    broker.set_publisher(publisher).await;

    let err = broker
        .request_approval(
            &Uuid::new_v4().to_string(),
            "mcpmux_pin_this_session",
            make_payload(),
        )
        .await
        .unwrap_err();
    assert!(matches!(err, MetaToolError::ApprovalTimedOut));
}

#[tokio::test]
async fn always_scope_persists_across_calls() {
    let broker = Arc::new(ApprovalBroker::new().with_timeout(Duration::from_millis(500)));
    let broker_clone = broker.clone();
    let client_id = Uuid::new_v4().to_string();

    let publisher: ApprovalPublisher = Arc::new(move |req| {
        let b = broker_clone.clone();
        async move {
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_millis(10)).await;
                b.respond(
                    &req.request_id,
                    &req.client_id,
                    &req.payload.tool_name,
                    ApprovalDecision::AlwaysForThisSessionAndClient,
                );
            });
            true
        }
        .boxed()
    });
    broker.set_publisher(publisher).await;

    // First call → dialog, returns AlwaysForThisSessionAndClient.
    let d1 = broker
        .request_approval(&client_id, "mcpmux_pin_this_session", make_payload())
        .await
        .unwrap();
    assert_eq!(d1, ApprovalDecision::AlwaysForThisSessionAndClient);

    // Second call → short-circuits via always-allow entry.
    let d2 = broker
        .request_approval(&client_id, "mcpmux_pin_this_session", make_payload())
        .await
        .unwrap();
    assert_eq!(d2, ApprovalDecision::AllowOnce);
}

#[tokio::test]
async fn always_allow_is_scoped_to_the_target_space() {
    let broker = Arc::new(ApprovalBroker::new().with_timeout(Duration::from_millis(500)));
    let prompts = Arc::new(AtomicUsize::new(0));
    broker
        .set_publisher(answering(
            &broker,
            ApprovalDecision::AlwaysForThisSessionAndClient,
            prompts.clone(),
        ))
        .await;
    let client_id = Uuid::new_v4().to_string();
    let (space_a, space_b) = (Uuid::new_v4(), Uuid::new_v4());

    for _ in 0..2 {
        broker
            .request_approval(
                &client_id,
                "mcpmux_manage_feature_set",
                payload_for(Some(space_a)),
            )
            .await
            .unwrap();
    }
    assert_eq!(prompts.load(Ordering::SeqCst), 1, "same Space: granted");

    broker
        .request_approval(
            &client_id,
            "mcpmux_manage_feature_set",
            payload_for(Some(space_b)),
        )
        .await
        .unwrap();
    assert_eq!(
        prompts.load(Ordering::SeqCst),
        2,
        "another Space prompts again"
    );
}

#[tokio::test]
async fn tokenless_connections_never_get_a_standing_grant() {
    let broker = Arc::new(ApprovalBroker::new().with_timeout(Duration::from_millis(500)));
    let prompts = Arc::new(AtomicUsize::new(0));
    broker
        .set_publisher(answering(
            &broker,
            ApprovalDecision::AlwaysForThisSessionAndClient,
            prompts.clone(),
        ))
        .await;

    for _ in 0..2 {
        broker
            .request_approval(
                ANONYMOUS_CLIENT_ID,
                "mcpmux_manage_feature_set",
                make_payload(),
            )
            .await
            .unwrap();
    }
    assert_eq!(prompts.load(Ordering::SeqCst), 2);
    assert!(broker.list_always_allow().is_empty());
}

#[tokio::test]
async fn always_allow_is_scoped_to_the_action_and_never_covers_delete() {
    let broker = Arc::new(ApprovalBroker::new().with_timeout(Duration::from_millis(500)));
    let prompts = Arc::new(AtomicUsize::new(0));
    broker
        .set_publisher(answering(
            &broker,
            ApprovalDecision::AlwaysForThisSessionAndClient,
            prompts.clone(),
        ))
        .await;
    let client_id = Uuid::new_v4().to_string();
    let space = Some(Uuid::new_v4());
    let request = |action: &'static str| {
        let broker = broker.clone();
        let client_id = client_id.clone();
        async move {
            broker
                .request_approval(
                    &client_id,
                    "mcpmux_manage_feature_set",
                    payload_with(Some(action), space),
                )
                .await
                .unwrap();
        }
    };

    request("create").await;
    request("create").await;
    assert_eq!(prompts.load(Ordering::SeqCst), 1, "create: granted");
    request("update").await;
    assert_eq!(
        prompts.load(Ordering::SeqCst),
        2,
        "update isn't covered by create"
    );
    request("delete").await;
    request("delete").await;
    assert_eq!(
        prompts.load(Ordering::SeqCst),
        4,
        "delete prompts every time"
    );
    let actions: Vec<_> = broker
        .list_always_allow()
        .into_iter()
        .map(|(_, _, action, _)| action)
        .collect();
    assert!(
        !actions.contains(&Some("delete".to_string())),
        "{actions:?}"
    );
}

#[tokio::test]
async fn workspace_binding_never_gets_a_standing_grant() {
    let broker = Arc::new(ApprovalBroker::new().with_timeout(Duration::from_millis(500)));
    let offered = Arc::new(std::sync::Mutex::new(Vec::new()));
    let seen = offered.clone();
    let responder = broker.clone();
    broker
        .set_publisher(Arc::new(move |req: ApprovalRequest| {
            seen.lock().unwrap().push(req.payload.allow_always);
            let broker = responder.clone();
            async move {
                tokio::spawn(async move {
                    broker.respond(
                        &req.request_id,
                        &req.client_id,
                        &req.payload.tool_name,
                        ApprovalDecision::AlwaysForThisSessionAndClient,
                    );
                });
                true
            }
            .boxed()
        }))
        .await;
    let client_id = Uuid::new_v4().to_string();

    for _ in 0..2 {
        broker
            .request_approval(
                &client_id,
                "mcpmux_bind_current_workspace",
                payload_for(Some(Uuid::nil())),
            )
            .await
            .unwrap();
    }
    broker
        .request_approval(
            &client_id,
            "mcpmux_pin_this_session",
            payload_for(Some(Uuid::nil())),
        )
        .await
        .unwrap();

    // The dialog is told not to offer "always" for binding, and nothing is
    // stored even if a response claims it.
    assert_eq!(*offered.lock().unwrap(), [false, false, true]);
    let tools: Vec<_> = broker
        .list_always_allow()
        .into_iter()
        .map(|(_, tool, _, _)| tool)
        .collect();
    assert_eq!(tools, ["mcpmux_pin_this_session"]);
}

#[tokio::test]
async fn revoking_removes_only_that_grant() {
    let broker = ApprovalBroker::new();
    let (a, b) = (Some(Uuid::new_v4()), Some(Uuid::new_v4()));
    broker.insert_always_allow("c", "mcpmux_manage_feature_set", Some("create"), a);
    broker.insert_always_allow("c", "mcpmux_manage_feature_set", Some("create"), b);
    assert!(broker.revoke_always_allow(&(
        "c".into(),
        "mcpmux_manage_feature_set".into(),
        Some("create".into()),
        a
    )));
    assert_eq!(broker.list_always_allow().len(), 1);
    assert_eq!(broker.list_always_allow()[0].3, b);
}

#[tokio::test]
async fn responses_to_unknown_requests_grant_nothing() {
    let broker = ApprovalBroker::new();
    assert!(!broker.respond(
        "no-such-request",
        "some-client",
        "mcpmux_manage_feature_set",
        ApprovalDecision::AlwaysForThisSessionAndClient,
    ));
    assert!(broker.list_always_allow().is_empty());
}
