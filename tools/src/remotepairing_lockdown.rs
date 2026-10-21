// Jackson Coxson
// The remotepairingdeviced control channel over USB lockdown

use idevice::{
    IdeviceService,
    provider::IdeviceProvider,
    remote_pairing::{RemotePairingLockdownService, RpPairingFile},
};
use jkcli::{CollectedArguments, JkArgument, JkCommand};

pub fn register() -> JkCommand {
    JkCommand::new()
        .help("Talk to the remotepairingdeviced control channel over USB lockdown")
        .with_subcommand(
            "info",
            JkCommand::new().help("Show a redacted control-channel handshake summary"),
        )
        .with_subcommand(
            "verify",
            JkCommand::new()
                .help("Pair-verify an existing RPPairing file against the device")
                .with_argument(
                    JkArgument::new()
                        .with_help("Path to the RPPairing file")
                        .required(true),
                )
                .with_argument(JkArgument::new().with_help("Hostname the file was created with")),
        )
        .with_subcommand(
            "pair",
            JkCommand::new()
                .help("Pair over USB, promptlessly, and write an RPPairing file")
                .with_argument(
                    JkArgument::new()
                        .with_help("Hostname to identify this computer")
                        .required(true),
                )
                .with_argument(
                    JkArgument::new()
                        .with_help("Path to save the pairing file")
                        .required(true),
                ),
        )
        .subcommand_required(true)
}

pub async fn main(arguments: &CollectedArguments, provider: Box<dyn IdeviceProvider>) {
    let (sub_name, sub_args) = arguments
        .first_subcommand()
        .expect("no subcommand passed, pass -h for help");
    let mut sub_args = sub_args.clone();

    match sub_name.as_str() {
        "info" => {
            let service = RemotePairingLockdownService::connect(&*provider)
                .await
                .unwrap_or_else(|_| panic!("no remotepairingdeviced lockdown service; detail redacted"));
            let mut client = service.into_client("idevice-rs-tools").unwrap_or_else(|_| panic!("no socket; detail redacted"));

            let handshake = client
                .attempt_pair_verify()
                .await
                .unwrap_or_else(|_| panic!("handshake failed; detail redacted"));
            let field_count = handshake.as_dictionary().map_or(0, |fields| fields.len());
            println!("handshake received: {field_count} top-level fields; payload redacted");
        }
        "verify" => {
            let path: String = sub_args
                .next_argument()
                .expect("pairing file path required");
            let hostname = sub_args
                .next_argument::<String>()
                .unwrap_or_else(|| "idevice-rs-tools".to_string());
            let mut pairing_file = RpPairingFile::read_from_file(&path)
                .await
                .unwrap_or_else(|_| panic!("failed to read the pairing file; detail redacted"));

            let service = RemotePairingLockdownService::connect(&*provider)
                .await
                .unwrap_or_else(|_| panic!("no remotepairingdeviced lockdown service; detail redacted"));
            let mut client = service.into_client(&hostname).unwrap_or_else(|_| panic!("no socket; detail redacted"));
            client
                .attempt_pair_verify()
                .await
                .unwrap_or_else(|_| panic!("handshake failed; detail redacted"));
            match client.validate_pairing(&mut pairing_file).await {
                Ok(()) => println!("the device recognizes this pairing record"),
                Err(e) => eprintln!("pair-verify failed: code={} sub_code={}", e.code(), e.sub_code()),
            }
        }
        "pair" => {
            let hostname: String = sub_args.next_argument().expect("hostname required");
            let output_path: String = sub_args.next_argument().expect("output path required");

            let service = RemotePairingLockdownService::connect(&*provider)
                .await
                .unwrap_or_else(|_| panic!("no remotepairingdeviced lockdown service; detail redacted"));
            let mut client = service.into_client(&hostname).unwrap_or_else(|_| panic!("no socket; detail redacted"));

            let mut pairing_file = RpPairingFile::generate(&hostname);
            // Over the already-trusted USB transport there is no Trust prompt,
            // so the PIN callback should never fire.
            match client
                .connect(&mut pairing_file, async || "000000".to_string())
                .await
            {
                Ok(()) => {
                    pairing_file
                        .write_to_file(&output_path)
                        .await
                        .unwrap_or_else(|_| panic!("failed to save the pairing file; detail redacted"));
                    println!("paired; pairing file saved; destination redacted");
                }
                Err(e) => eprintln!("pairing failed: code={} sub_code={}", e.code(), e.sub_code()),
            }
        }
        _ => unreachable!(),
    }
}
