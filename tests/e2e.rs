use std::{
    collections::{BTreeMap, HashSet},
    path::Path,
};

use anyhow::Result;
use common::{setup_command, test_data};
use policy_evaluator::{policy_fetcher, policy_metadata};
use predicates::{prelude::*, str::contains, str::is_empty};
use rstest::rstest;
use tempfile::tempdir;
use testcontainers::{core::WaitFor, runners::SyncRunner};

mod common;

const POLICIES: &[&str] = &[
    // SHA: 01690a10f9c3
    "registry://ghcr.io/kubewarden/tests/pod-privileged:v0.2.5",
    // SHA: 828617a7cf3e
    "registry://ghcr.io/kubewarden/tests/safe-labels:v0.1.13",
];

fn pull_policies(path: &Path, policies: &[&str]) {
    for policy in policies {
        let mut cmd = setup_command(path);
        cmd.arg("pull").arg(policy);

        cmd.assert().success();
    }
}

#[test]
fn test_policies_empty() {
    let tempdir = tempdir().unwrap();

    let mut cmd = setup_command(tempdir.path());
    cmd.arg("policies");

    cmd.assert().success();
    cmd.assert().stdout("");
}

#[test]
fn test_policies() {
    let tempdir = tempdir().unwrap();
    pull_policies(tempdir.path(), POLICIES);

    let mut cmd = setup_command(tempdir.path());
    cmd.arg("policies");

    cmd.assert().success();
    cmd.assert()
        .stdout(contains("pod-privileged"))
        .stdout(contains("v0.2.5"))
        .stdout(contains("safe-labels"))
        .stdout(contains("v0.1.13"));
}

#[rstest]
#[case::https(
    "https://github.com/kubewarden/pod-privileged-policy/releases/download/v0.2.5/policy.wasm"
)]
#[case::registry("registry://ghcr.io/kubewarden/tests/pod-privileged:v0.2.5")]
fn test_pull(#[case] uri: &str) {
    let tempdir = tempdir().unwrap();

    let mut cmd = setup_command(tempdir.path());
    cmd.arg("pull").arg(uri);

    cmd.assert().success();

    let mut cmd = setup_command(tempdir.path());
    cmd.arg("policies");

    cmd.assert().success();
    cmd.assert().stdout(contains(uri));
}

#[test]
fn test_pull_registry_no_tag() {
    let tempdir = tempdir().unwrap();

    let mut cmd = setup_command(tempdir.path());
    cmd.arg("pull")
        .arg("registry://ghcr.io/kubewarden/tests/sleeping-policy");

    cmd.assert().success();

    let mut cmd = setup_command(tempdir.path());
    cmd.arg("policies");

    cmd.assert().success();
    cmd.assert().stdout(contains(
        "registry://ghcr.io/kubewarden/tests/sleeping-policy:latest",
    ));
}

#[rstest]
#[case::allowed("unprivileged-pod.json", true)]
#[case::rejected("privileged-pod.json", false)]
#[case::admission_review_allowed("unprivileged-pod-admission-review.json", true)]
#[case::admission_review_rejected("privileged-pod-admission-review.json", false)]
fn test_run(#[case] request: &str, #[case] allowed: bool) {
    let tempdir = tempdir().unwrap();
    pull_policies(tempdir.path(), POLICIES);

    let mut cmd = setup_command(tempdir.path());
    cmd.arg("run")
        .arg("--request-path")
        .arg(test_data(request))
        .arg("registry://ghcr.io/kubewarden/tests/pod-privileged:v0.2.5");

    cmd.assert().success();
    cmd.assert()
        .stdout(contains(format!("\"allowed\":{}", allowed)));
}

#[rstest]
#[case::allowed(
    "registry://ghcr.io/kubewarden/tests/context-aware-policy-demo:v0.1.0",
    "context-aware-policy-request-pod-creation-all-labels.json",
    "context-aware-demo-namespace-found.yml",
    true
)]
#[case::rejected(
    "registry://ghcr.io/kubewarden/tests/context-aware-policy-demo:v0.1.0",
    "context-aware-policy-request-pod-creation-all-labels.json",
    "context-aware-demo-namespace-not-found.yml",
    false
)]
#[case::gatekeeper_allowed(
    "registry://ghcr.io/kubewarden/tests/unique-ingress-policy:v0.1.3",
    "ingress.json",
    "context-aware-unique-ingress-no-duplicate.yml",
    true
)]
#[case::gatekeeper_rejected(
    "registry://ghcr.io/kubewarden/tests/unique-ingress-policy:v0.1.3",
    "ingress.json",
    "context-aware-unique-ingress-duplicate.yml",
    false
)]
fn test_run_context(
    #[case] policy_uri: &str,
    #[case] request: &str,
    #[case] session: &str,
    #[case] allowed: bool,
) {
    let tempdir = tempdir().unwrap();
    pull_policies(tempdir.path(), POLICIES);

    let session_path = test_data(format!("host-capabilities-sessions/{}", session).as_str());
    let mut cmd = setup_command(tempdir.path());

    cmd.arg("run")
        .arg("--allow-context-aware")
        .arg("--request-path")
        .arg(test_data(request))
        .arg("--replay-host-capabilities-interactions")
        .arg(session_path)
        .arg(policy_uri);

    cmd.assert().success();
    cmd.assert()
        .stdout(contains(format!("\"allowed\":{}", allowed)));
}

#[test]
fn test_run_sha_prefix() {
    let tempdir = tempdir().unwrap();
    pull_policies(tempdir.path(), POLICIES);

    let mut cmd = setup_command(tempdir.path());
    cmd.arg("run")
        .arg("--request-path")
        .arg(test_data("unprivileged-pod.json"))
        .arg("0169");

    cmd.assert().success();
    cmd.assert().stdout(contains("\"allowed\":true"));
}

#[test]
fn test_run_remote() {
    let tempdir = tempdir().unwrap();

    let mut cmd = setup_command(tempdir.path());
    cmd.arg("run")
        .arg("--request-path")
        .arg(test_data("unprivileged-pod.json"))
        .arg("registry://ghcr.io/kubewarden/tests/pod-privileged:v0.2.5");

    cmd.assert().success();
    cmd.assert().stdout(contains("\"allowed\":true"));
}

#[test]
fn test_run_raw() {
    let tempdir = tempdir().unwrap();

    let mut cmd = setup_command(tempdir.path());
    cmd.arg("run")
        .arg("--request-path")
        .arg(test_data("raw.json"))
        .arg("--settings-json")
        .arg(r#"{"defaultResource": "rice","forbiddenResources": ["banana","apple"]}"#)
        .arg("registry://ghcr.io/kubewarden/tests/raw-mutation-policy:v0.1.0");

    cmd.assert().success();
    cmd.assert().stdout(contains("\"allowed\":true"));
    cmd.assert().stdout(contains("\"patchType\":\"JSONPatch\""));
}

#[test]
fn test_run_raw_non_annotated() {
    let tempdir = tempdir().unwrap();

    let mut cmd = setup_command(tempdir.path());
    cmd.arg("run")
        .arg("--raw")
        .arg("--request-path")
        .arg(test_data("raw.json"))
        .arg("registry://ghcr.io/kubewarden/tests/raw-mutation-non-annotated-policy:v0.1.0");

    cmd.assert().success();
    cmd.assert().stdout(contains("\"allowed\":true"));
    cmd.assert().stdout(contains("\"patchType\":\"JSONPatch\""));
}

#[rstest]
fn test_bench() {
    let tempdir = tempdir().unwrap();

    let mut cmd = setup_command(tempdir.path());
    cmd.arg("bench")
        .arg("--warm-up-time")
        .arg("1")
        .arg("--measurement-time")
        .arg("1")
        .arg("--num-resamples")
        .arg("2")
        .arg("--num-samples")
        .arg("2")
        .arg("--request-path")
        .arg(test_data("unprivileged-pod.json"))
        .arg("registry://ghcr.io/kubewarden/tests/pod-privileged:v0.2.5");

    cmd.assert().success();
    cmd.assert()
        .stdout(contains("validate").and(contains("warming up")));
}

#[rstest]
#[case(
    "registry://ghcr.io/kubewarden/tests/pod-privileged:v0.2.5",
    "5ddb9b97ac5e466ae81c34b856d526eed784784024133ba67b1a907f63dfa0a2"
)]
#[case(
    "ghcr.io/kubewarden/tests/safe-labels:v0.1.13",
    "6b6330115f78b4007bbd0b5b342825770e694f561ee74539ed68865d7b172341"
)]
fn test_digest(#[case] uri: &str, #[case] expected_sha: &str) {
    let tempdir = tempdir().unwrap();
    let mut cmd = setup_command(tempdir.path());

    cmd.arg("digest").arg(uri);

    cmd.assert().success();
    cmd.assert().stdout(contains(expected_sha));
}

#[rstest]
#[case(
    "registry://ghcr.io/kubewarden/tests/pod-privileged:v0.2.5",
    true,
    is_empty()
)]
#[case::sha_prefix("0169", true, is_empty())]
#[case::non_existing("non-existing", false, contains("Cannot find policy"))]
fn test_rm(
    #[case] policy_ref: &str,
    #[case] success: bool,
    #[case] predicate: impl predicates::str::PredicateStrExt,
) {
    let tempdir = tempdir().unwrap();
    pull_policies(tempdir.path(), POLICIES);

    let mut cmd = setup_command(tempdir.path());
    cmd.arg("rm").arg(policy_ref);

    if success {
        cmd.assert().success();
        cmd.assert().stdout(predicate);
    } else {
        cmd.assert().failure();
        cmd.assert().stderr(predicate);
    }

    let mut cmd = setup_command(tempdir.path());
    cmd.arg("policies").assert().success();
    cmd.assert().stdout(contains(policy_ref).not());
}

#[test]
fn test_save_and_load() {
    let tempdir = tempdir().unwrap();
    pull_policies(tempdir.path(), POLICIES);

    let mut cmd = setup_command(tempdir.path());
    cmd.arg("save").arg("--output").arg("policies.tar.gz");
    for policy in POLICIES {
        cmd.arg(policy);
    }
    cmd.assert().success();

    for policy in POLICIES {
        let mut cmd = setup_command(tempdir.path());
        cmd.arg("rm").arg(policy);
        cmd.assert().success();
    }

    let mut cmd = setup_command(tempdir.path());
    cmd.arg("load").arg("--input").arg("policies.tar.gz");
    cmd.assert().success();

    let mut cmd = setup_command(tempdir.path());
    cmd.arg("policies");
    cmd.assert().success();
    for policy in POLICIES {
        cmd.assert().stdout(contains(*policy));
    }
}

#[test]
fn test_push() {
    let registry_image = testcontainers::GenericImage::new("docker.io/library/registry", "2")
        .with_wait_for(WaitFor::message_on_stderr("listening on "));
    let testcontainer = registry_image
        .start()
        .expect("Failed to start registry container");
    let port = testcontainer
        .get_host_port_ipv4(5000)
        .expect("Failed to get port");

    let tempdir = tempdir().unwrap();
    pull_policies(tempdir.path(), POLICIES);

    let sources_yaml = format!(
        r#"
        insecure_sources:
            - "localhost:{}"
        "#,
        port
    );
    std::fs::write(tempdir.path().join("sources.yml"), sources_yaml).unwrap();

    let target_image = format!(
        "registry://localhost:{}/my-pod-privileged-policy:v0.1.10",
        port
    );

    let mut cmd = setup_command(tempdir.path());
    cmd.arg("push")
        .arg("--sources-path")
        .arg("sources.yml")
        .arg("registry://ghcr.io/kubewarden/tests/pod-privileged:v0.2.5")
        .arg(&target_image);
    cmd.assert().success();

    let wasm_annotations = get_wasm_annotations(
        tempdir.path(),
        "registry://ghcr.io/kubewarden/tests/pod-privileged:v0.2.5",
    )
    .expect("cannot get wasm annotations");

    let sources = policy_fetcher::sources::Sources {
        insecure_sources: HashSet::from([format!("localhost:{}", port)]),
        ..Default::default()
    };
    let manifest_annotations = get_manifest_annotations(
        format!(
            "registry://localhost:{}/my-pod-privileged-policy:v0.1.10",
            port
        )
        .as_str(),
        &sources,
    )
    .expect("cannot get OCI manifest annotations");

    for (wasm_key, wasm_value) in &wasm_annotations {
        if wasm_value.lines().count() > 1 {
            continue;
        }

        let manifest_value = manifest_annotations
            .get(wasm_key)
            .unwrap_or_else(|| panic!("missing annotation {}", wasm_key));
        assert_eq!(wasm_value, manifest_value,);
    }

    let mut cmd = setup_command(tempdir.path());
    cmd.arg("pull")
        .arg("--sources-path")
        .arg("sources.yml")
        .arg(format!(
            "registry://localhost:{}/my-pod-privileged-policy:v0.1.10",
            port
        ));
    cmd.assert().success();

    let mut cmd = setup_command(tempdir.path());
    cmd.arg("policies");
    cmd.assert().success();
    cmd.assert()
        .stdout(contains("my-pod-privileged-policy:v0.1.10"));
}

#[rstest]
#[case::pull_policies_before_scaffold(true)]
#[case::pull_policies_on_demand(false)]
fn test_scaffold_manifest(#[case] pull_policies_before: bool) {
    let tempdir = tempdir().unwrap();
    if pull_policies_before {
        pull_policies(tempdir.path(), POLICIES);
    }

    let mut cmd = setup_command(tempdir.path());
    cmd.arg("scaffold")
        .arg("manifest")
        .arg("--settings-json")
        .arg(r#"{"denied_labels": ["foo", "bar"]}"#)
        .arg("-t")
        .arg("ClusterAdmissionPolicy")
        .arg("registry://ghcr.io/kubewarden/tests/safe-labels:v0.1.13");

    cmd.assert().success();
    cmd.assert().stdout(contains(
        "registry://ghcr.io/kubewarden/tests/safe-labels:v0.1.13",
    ));
    cmd.assert().stdout(contains("denied_labels"));
    cmd.assert().stdout(contains("foo"));
    cmd.assert().stdout(contains("bar"));
    cmd.assert().stdout(contains("ClusterAdmissionPolicy"));
}

#[rstest]
#[case::latest_cel_policy(
    Some("vap/vap-with-variables.yml"),
    Some("vap/vap-binding.yml"),
    Some("registry.example.com/cel-policy"),
    true,
    contains("module: registry.example.com/cel-policy"),
    contains("Using the 'latest' version of the CEL policy")
)]
#[case::cel_policy_version_not_provided(
    Some("vap/vap-with-variables.yml"),
    Some("vap/vap-binding.yml"),
    None,
    true,
    contains("module: ghcr.io/kubewarden/policies/cel-policy:latest"),
    contains("Using the 'latest' version of the CEL policy")
)]
#[case::custom_cel_policy(
    Some("vap/vap-with-variables.yml"),
    Some("vap/vap-binding.yml"),
    Some("ghcr.io/kubewarden/tests/cel-policy:1.0.0"),
    true,
    contains("module: ghcr.io/kubewarden/tests/cel-policy:1.0.0"),
    is_empty()
)]
#[case::missing_policy(
    None,
    Some("vap/vap-binding.yml"),
    None,
    false,
    is_empty(),
    contains("the following required arguments were not provided")
)]
#[case::missing_binding(
    Some("vap/vap-with-variables.yml"),
    None,
    None,
    false,
    is_empty(),
    contains("the following required arguments were not provided")
)]
fn test_scaffold_from_vap(
    #[case] vap_path: Option<&str>,
    #[case] vap_binding: Option<&str>,
    #[case] cel_policy_module: Option<&str>,
    #[case] success: bool,
    #[case] stdout_predicate: impl predicates::str::PredicateStrExt,
    #[case] stderr_predicate: impl predicates::str::PredicateStrExt,
) {
    let tempdir = tempdir().unwrap();

    let mut cmd = setup_command(tempdir.path());
    cmd.arg("scaffold").arg("vap");

    if let Some(vap) = vap_path {
        cmd.arg("--policy").arg(test_data(vap));
    }
    if let Some(vap_binding) = vap_binding {
        cmd.arg("--binding").arg(test_data(vap_binding));
    }
    if let Some(module) = cel_policy_module {
        cmd.arg("--cel-policy").arg(module);
    }

    if success {
        cmd.assert().success();
    } else {
        cmd.assert().failure();
    }

    cmd.assert().stdout(stdout_predicate);
    cmd.assert().stderr(stderr_predicate);
}

#[rstest]
#[case::correct("rego-annotate/metadata-correct.yml", true, is_empty())]
#[case::wrong(
    "rego-annotate/metadata-wrong.yml",
    false,
    contains("Error: Wrong value inside of policy's metadata for 'executionMode'. This policy has been created using Rego")
)]
fn test_annotate_rego(
    #[case] metadata_path: &str,
    #[case] success: bool,
    #[case] predicate: impl predicates::str::PredicateStrExt,
) {
    let tempdir = tempdir().unwrap();

    let mut cmd = setup_command(tempdir.path());
    cmd.arg("annotate")
        .arg("-m")
        .arg(test_data(metadata_path))
        .arg(test_data("rego-annotate/no-default-namespace-rego.wasm"))
        .arg("-o")
        .arg("annotated-policy.wasm");

    if success {
        cmd.assert().success();
        cmd.assert().stdout(predicate);
    } else {
        cmd.assert().failure();
        cmd.assert().stderr(predicate);
    }
}

#[rstest]
#[case::show_signatures(true)]
#[case::hide_signatures(false)]
fn test_inspect_policy_yml_output(#[case] show_signatures: bool) {
    let uri = "registry://ghcr.io/kubewarden/tests/pod-privileged:v0.2.5";

    let tempdir = tempdir().unwrap();

    let mut cmd = setup_command(tempdir.path());
    cmd.arg("pull").arg(uri);

    cmd.assert().success();

    let mut cmd = setup_command(tempdir.path());
    cmd.arg("inspect").arg("-o").arg("yaml");

    if show_signatures {
        cmd.arg("--show-signatures");
    }
    cmd.arg(uri);

    cmd.assert().success();
    let report: serde_yaml::Mapping = serde_yaml::from_slice(&cmd.assert().get_output().stdout)
        .expect("a valid yaml document was expected");
    assert_eq!(show_signatures, report.contains_key("signatures"))
}

fn get_wasm_annotations(dir: &Path, oci_ref: &str) -> Result<BTreeMap<String, String>> {
    let mut cmd = setup_command(dir);
    cmd.arg("inspect").arg(oci_ref).arg("-o").arg("yaml");
    let metadata: policy_metadata::Metadata =
        serde_yaml::from_slice(&cmd.assert().success().get_output().stdout)
            .expect("cannot deserialize 'kwctl inspect -o yaml'");

    Ok(metadata.annotations.unwrap_or_default())
}

fn get_manifest_annotations(
    oci_ref: &str,
    sources: &policy_fetcher::sources::Sources,
) -> Result<BTreeMap<String, String>> {
    use policy_fetcher::oci_client::manifest::OciManifest;

    let registry = policy_fetcher::registry::Registry::new();

    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        match registry.manifest(oci_ref, Some(sources)).await? {
            OciManifest::Image(manifest) => Ok(manifest.annotations.unwrap_or_default()),
            _ => Err(anyhow::anyhow!("not an image manifest")),
        }
    })
}
