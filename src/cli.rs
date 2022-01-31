use clap::{crate_authors, crate_description, crate_name, crate_version, App, AppSettings, Arg};
use itertools::Itertools;
use lazy_static::lazy_static;
use policy_evaluator::burrego::opa::builtins as opa_builtins;
use std::path::PathBuf;

lazy_static! {
    static ref VERSION_AND_BUILTINS: String = {
        let builtins: String = opa_builtins::get_builtins()
            .keys()
            .sorted()
            .map(|builtin| format!("  - {}", builtin))
            .join("\n");

        format!(
            "{}\n\nOpen Policy Agent/Gatekeeper implemented builtins:\n{}",
            crate_version!(),
            builtins,
        )
    };
    static ref SIGSTORE_TARGETS_PATH: PathBuf = {
        directories::BaseDirs::new()
            .unwrap_or_else(|| panic!("not possible to build base dirs"))
            .home_dir()
            .join(".sigstore")
            .join("root")
            .join("targets")
    };
    pub(crate) static ref SIGSTORE_FULCIO_CERT_PATH: PathBuf =
        SIGSTORE_TARGETS_PATH.join("fulcio.crt.pem");
    pub(crate) static ref SIGSTORE_REKOR_PUBLIC_KEY_PATH: PathBuf =
        SIGSTORE_TARGETS_PATH.join("rekor.pub");
}

pub fn build_cli() -> clap::App<'static> {
    App::new(crate_name!())
        .version(crate_version!())
        .author(crate_authors!())
        .about(crate_description!())
        .arg(Arg::new("verbose").short('v').help("Increase verbosity"))
        .subcommand(
            App::new("policies")
                .about("Lists all downloaded policies")
        )
        .subcommand(
            App::new("pull")
                .about("Pulls a Kubewarden policy from a given URI")
                .arg(
                    Arg::new("docker-config-json-path")
                    .long("docker-config-json-path")
                    .takes_value(true)
                    .help("Path to a Docker config.json-like path. Can be used to indicate registry authentication details")
                )
                .arg(
                    Arg::new("sources-path")
                    .long("sources-path")
                    .takes_value(true)
                    .help("YAML file holding source information (https, registry insecure hosts, custom CA's...)")
                )
                .arg(
                    Arg::new("verification-key")
                    .short('k')
                    .long("verification-key")
                    .multiple_occurrences(true)
                    .number_of_values(1)
                    .takes_value(true)
                    .help("Path to key used to verify the policy. Can be repeated multiple times")
                )
                .arg(
                    Arg::new("fulcio-cert-path")
                    .long("fulcio-cert-path")
                    .takes_value(true)
                    .help("Path to the Fulcio certificate")
                )
                .arg(
                    Arg::new("rekor-public-key-path")
                    .long("rekor-public-key-path")
                    .takes_value(true)
                    .help("Path to the Rekor public key")
                )
                .arg(
                    Arg::new("verification-annotation")
                    .short('a')
                    .long("verification-annotation")
                    .multiple_occurrences(true)
                    .number_of_values(1)
                    .takes_value(true)
                    .help("Annotation in key=value format. Can be repeated multiple times")
                )
                .arg(
                    Arg::new("output-path")
                    .short('o')
                    .long("output-path")
                    .takes_value(true)
                    .help("Output file. If not provided will be downloaded to the Kubewarden store")
                )
                .arg(
                    Arg::new("uri")
                        .required(true)
                        .index(1)
                        .help("Policy URI. Supported schemes: registry://, https://, file://")
                )
        )
        .subcommand(
            App::new("verify")
                .about("Verify a Kubewarden policy from a given URI using Sigstore")
                .arg(
                    Arg::new("docker-config-json-path")
                    .long("docker-config-json-path")
                    .takes_value(true)
                    .help("Path to a Docker config.json-like path. Can be used to indicate registry authentication details")
                )
                .arg(
                    Arg::new("sources-path")
                    .long("sources-path")
                    .takes_value(true)
                    .help("YAML file holding source information (https, registry insecure hosts, custom CA's...)")
                )
                .arg(
                    Arg::new("verification-key")
                    .short('k')
                    .long("verification-key")
                    .multiple_occurrences(true)
                    .number_of_values(1)
                    .takes_value(true)
                    .required(true)
                    .help("Path to key used to verify the policy. Can be repeated multiple times")
                )
                .arg(
                    Arg::new("fulcio-cert-path")
                    .long("fulcio-cert-path")
                    .takes_value(true)
                    .help("Path to the Fulcio certificate")
                )
                .arg(
                    Arg::new("rekor-public-key-path")
                    .long("rekor-public-key-path")
                    .takes_value(true)
                    .help("Path to the Rekor public key")
                )
                .arg(
                    Arg::new("verification-annotation")
                    .short('a')
                    .long("verification-annotation")
                    .multiple_occurrences(true)
                    .number_of_values(1)
                    .takes_value(true)
                    .help("Annotation in key=value format. Can be repeated multiple times")
                )
                .arg(
                    Arg::new("uri")
                        .required(true)
                        .index(1)
                        .help("Policy URI. Supported schemes: registry://")
                )
        )
        .subcommand(
            App::new("push")
                .about("Pushes a Kubewarden policy to an OCI registry")
                .arg(
                    Arg::new("docker-config-json-path")
                    .long("docker-config-json-path")
                    .takes_value(true)
                    .help("Path to a Docker config.json-like path. Can be used to indicate registry authentication details")
                )
                .arg(
                    Arg::new("sources-path")
                    .long("sources-path")
                    .takes_value(true)
                    .help("YAML file holding source information (https, registry insecure hosts, custom CA's...)")
                )
                .arg(
                    Arg::new("force")
                    .short('f')
                    .long("force")
                    .help("Push also a policy that is not annotated")
                )
                .arg(
                    Arg::new("output")
                    .long("output")
                    .short('o')
                    .takes_value(true)
                    .possible_values(&["text", "json"])
                    .default_value("text")
                    .help("Output format")
                )
               .arg(
                    Arg::new("policy")
                        .required(true)
                        .index(1)
                        .help("Policy to push. Can be the path to a local file, or a policy URI")
                )
               .arg(
                    Arg::new("uri")
                        .required(true)
                        .index(2)
                        .help("Policy URI. Supported schemes: registry://")
                )
        )
        .subcommand(
            App::new("rm")
                .about("Removes a Kubewarden policy from the store")
                .arg(
                    Arg::new("uri")
                        .required(true)
                        .index(1)
                        .help("Policy URI")
                )
        )
        .subcommand(
            App::new("run")
                .about("Runs a Kubewarden policy from a given URI")
                .arg(
                    Arg::new("docker-config-json-path")
                    .long("docker-config-json-path")
                    .takes_value(true)
                    .help("Path to a Docker config.json-like path. Can be used to indicate registry authentication details")
                )
                .arg(
                    Arg::new("sources-path")
                    .long("sources-path")
                    .takes_value(true)
                    .help("YAML file holding source information (https, registry insecure hosts, custom CA's...)")
                )
                .arg(
                    Arg::new("request-path")
                    .long("request-path")
                    .short('r')
                    .required(true)
                    .takes_value(true)
                    .help("File containing the Kubernetes admission request object in JSON format")
                )
                .arg(
                    Arg::new("settings-path")
                    .long("settings-path")
                    .short('s')
                    .takes_value(true)
                    .help("File containing the settings for this policy")
                )
                .arg(
                    Arg::new("settings-json")
                    .long("settings-json")
                    .takes_value(true)
                    .help("JSON string containing the settings for this policy")
                )
                .arg(
                    Arg::new("verification-key")
                    .short('k')
                    .long("verification-key")
                    .multiple_occurrences(true)
                    .number_of_values(1)
                    .takes_value(true)
                    .help("Path to key used to verify the policy. Can be repeated multiple times")
                )
                .arg(
                    Arg::new("fulcio-cert-path")
                    .long("fulcio-cert-path")
                    .takes_value(true)
                    .help("Path to the Fulcio certificate")
                )
                .arg(
                    Arg::new("rekor-public-key-path")
                    .long("rekor-public-key-path")
                    .takes_value(true)
                    .help("Path to the Rekor public key")
                )
                .arg(
                    Arg::new("verification-annotation")
                    .short('a')
                    .long("verification-annotation")
                    .multiple_occurrences(true)
                    .number_of_values(1)
                    .takes_value(true)
                    .help("Annotation in key=value format. Can be repeated multiple times")
                )
                .arg(
                    Arg::new("execution-mode")
                    .long("execution-mode")
                    .short('e')
                    .takes_value(true)
                    .possible_values(&["opa","gatekeeper", "kubewarden"])
                    .help("The runtime to use to execute this policy")
                )
                .arg(
                    Arg::new("uri")
                        .required(true)
                        .index(1)
                        .help("Policy URI. Supported schemes: registry://, https://, file://. If schema is omitted, file:// is assumed, rooted on the current directory")
                )
        )
        .subcommand(
            App::new("annotate")
                .about("Add Kubewarden metadata to a WebAssembly module")
                .arg(
                    Arg::new("metadata-path")
                    .long("metadata-path")
                    .short('m')
                    .required(true)
                    .takes_value(true)
                    .help("File containing the metadata")
                )
                .arg(
                    Arg::new("output-path")
                    .long("output-path")
                    .short('o')
                    .required(true)
                    .takes_value(true)
                    .help("Output file")
                )
                .arg(
                    Arg::new("wasm-path")
                    .required(true)
                    .index(1)
                    .help("Path to WebAssembly module to be annotated")
                )
        )
        .subcommand(
            App::new("inspect")
                .about("Inspect Kubewarden policy")
                .arg(
                    Arg::new("output")
                    .long("output")
                    .short('o')
                    .takes_value(true)
                    .possible_values(&["yaml"])
                    .help("Output format")
                )
                .arg(
                    Arg::new("uri")
                        .required(true)
                        .index(1)
                        .help("Policy URI. Supported schemes: registry://, https://, file://")
                )
        )
        .subcommand(
            App::new("manifest")
                .about("Scaffold a Kubernetes resource")
                .arg(
                    Arg::new("settings-path")
                    .long("settings-path")
                    .short('s')
                    .takes_value(true)
                    .help("File containing the settings for this policy")
                )
                .arg(
                    Arg::new("settings-json")
                    .long("settings-json")
                    .takes_value(true)
                    .help("JSON string containing the settings for this policy")
                )
                .arg(
                    Arg::new("type")
                    .long("type")
                    .short('t')
                    .required(true)
                    .takes_value(true)
                    .possible_values(&["ClusterAdmissionPolicy"])
                    .help("Kubewarden Custom Resource type")
                )
                .arg(
                    Arg::new("uri")
                        .required(true)
                        .index(1)
                        .help("Policy URI. Supported schemes: registry://, https://, file://")
                )
                .arg(
                    Arg::new("title")
                        .long("title")
                        .takes_value(true)
                        .help("Policy title")
                )
        )
        .subcommand(
            App::new("completions")
                .about("Generate shell completions")
                .arg(
                    Arg::new("shell")
                    .long("shell")
                    .short('s')
                    .takes_value(true)
                    .required(true)
                    .possible_values(&["bash", "fish", "zsh", "elvish", "powershell"])
                    .help("Shell type")
                )
        )
        .subcommand(
            App::new("digest")
                .about("Fetch the digest of its OCI manifest")
                .arg(
                    Arg::new("uri")
                        .required(true)
                        .index(1)
                        .help("Policy URI")
                )
                .arg(
                    Arg::new("sources-path")
                        .long("sources-path")
                        .takes_value(true)
                        .help("YAML file holding source information (https, registry insecure hosts, custom CA's...)")
                )
                .arg(
                    Arg::new("docker-config-json-path")
                        .long("docker-config-json-path")
                        .takes_value(true)
                        .help("Path to a Docker config.json-like path. Can be used to indicate registry authentication details")
                )
        )
        .long_version(VERSION_AND_BUILTINS.as_str())
        .setting(AppSettings::SubcommandRequiredElseHelp)
}
