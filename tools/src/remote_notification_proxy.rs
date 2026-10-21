// Jackson Coxson

use idevice::{
    IdeviceService, ReadWrite, RsdService, core_device_proxy::CoreDeviceProxy,
    notification_proxy::RemoteNotificationProxyClient, provider::IdeviceProvider,
    rsd::RsdHandshake,
};
use jkcli::{CollectedArguments, JkArgument, JkCommand, JkFlag};

pub const INSECURE_REMOTE_NOTIFICATION_PROXY_SERVICE: &str =
    "com.apple.mobile.insecure_notification_proxy.remote";

pub fn register() -> JkCommand {
    JkCommand::new()
        .help("Post and observe Darwin notifications over the RemoteXPC notification proxy")
        .with_subcommand(
            "post",
            JkCommand::new()
                .help("Post a notification on the device")
                .with_argument(
                    JkArgument::new()
                        .with_help("notification name")
                        .required(true),
                ),
        )
        .with_subcommand(
            "observe",
            JkCommand::new()
                .help("Observe notifications and stream them as they fire")
                .with_argument(
                    JkArgument::new()
                        .with_help("notification name (repeatable)")
                        .required(true),
                )
                .with_argument(JkArgument::new().with_help("more notification names"))
                .with_argument(JkArgument::new().with_help("more notification names"))
                .with_argument(JkArgument::new().with_help("more notification names")),
        )
        .with_flag(
            JkFlag::new("insecure").with_help("Use the insecure relay meant for untrusted clients"),
        )
        .subcommand_required(true)
}

pub async fn main(arguments: &CollectedArguments, provider: Box<dyn IdeviceProvider>) {
    let (sub_name, sub_args) = arguments
        .first_subcommand()
        .expect("no subcommand passed, pass -h for help");
    let mut sub_args = sub_args.clone();
    let insecure = arguments.has_flag("insecure") || sub_args.has_flag("insecure");

    let proxy = CoreDeviceProxy::connect(&*provider)
        .await
        .unwrap_or_else(|_| panic!("no core device proxy; detail redacted"));
    let rsd_port = proxy.tunnel_info().server_rsd_port;
    let adapter = proxy.create_software_tunnel().unwrap_or_else(|_| panic!("no software tunnel; detail redacted"));
    let mut adapter = adapter.to_async_handle();
    let stream = adapter.connect(rsd_port).await.unwrap_or_else(|_| panic!("no RSD connect; detail redacted"));
    let mut handshake = RsdHandshake::new(stream).await.unwrap_or_else(|_| panic!("operation failed; detail redacted"));

    let mut client = if insecure {
        let port = handshake
            .services
            .get(INSECURE_REMOTE_NOTIFICATION_PROXY_SERVICE)
            .map(|s| s.port)
            .expect("the device doesn't advertise the insecure remote notification proxy");
        let stream = adapter.connect(port).await.unwrap_or_else(|_| panic!("no service connect; detail redacted"));
        RemoteNotificationProxyClient::new(Box::new(stream) as Box<dyn ReadWrite>)
            .await
            .unwrap_or_else(|_| panic!("no insecure notification proxy; detail redacted"))
    } else {
        RemoteNotificationProxyClient::connect_rsd(&mut adapter, &mut handshake)
            .await
            .unwrap_or_else(|_| panic!("no remote notification proxy; detail redacted"))
    };

    match sub_name.as_str() {
        "post" => {
            let name: String = sub_args
                .next_argument()
                .expect("notification name required");
            client
                .post_notification(&name)
                .await
                .unwrap_or_else(|_| panic!("failed to post the notification; detail redacted"));
            println!("posted {name}");
        }
        "observe" => {
            let mut names = Vec::new();
            while let Some(name) = sub_args.next_argument::<String>() {
                names.push(name);
            }
            for name in &names {
                client
                    .observe_notification(name.as_str())
                    .await
                    .unwrap_or_else(|_| panic!("failed to observe the notification; detail redacted"));
                println!("observing {name}");
            }

            loop {
                match client.receive_notification().await {
                    Ok(name) => println!("{name}"),
                    Err(e) => {
                        eprintln!("stream ended: code={} sub_code={}", e.code(), e.sub_code());
                        return;
                    }
                }
            }
        }
        _ => unreachable!(),
    }
}
