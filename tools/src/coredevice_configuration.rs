// Jackson Coxson

use idevice::{
    IdeviceService, RsdService,
    core_device::{ConfigurationServiceClient, UserInterfaceStyle},
    core_device_proxy::CoreDeviceProxy,
    provider::IdeviceProvider,
    rsd::RsdHandshake,
};
use jkcli::{CollectedArguments, JkArgument, JkCommand, JkFlag};

pub fn register() -> JkCommand {
    let toggle = |help: &'static str| {
        JkCommand::new()
            .help(help)
            .with_argument(JkArgument::new().with_help("on or off; omit to read the current value"))
    };

    JkCommand::new()
        .help("Read and write appearance and accessibility knobs over CoreDevice")
        .with_subcommand(
            "appearance",
            JkCommand::new()
                .help("Get or set the light/dark appearance")
                .with_argument(
                    JkArgument::new().with_help("dark or light; omit to read the current style"),
                ),
        )
        .with_subcommand(
            "opacity",
            JkCommand::new()
                .help("Set the system liquid-glass opacity")
                .with_argument(
                    JkArgument::new()
                        .with_help("opacity, 0.0 to 1.0")
                        .required(true),
                ),
        )
        .with_subcommand(
            "color-filter",
            JkCommand::new()
                .help("Get or set the accessibility color filter")
                .with_argument(JkArgument::new().with_help("on or off; omit to read the state"))
                .with_argument(JkArgument::new().with_help("filter preset, e.g. Protanopia"))
                .with_argument(JkArgument::new().with_help("intensity, 0.0 to 1.0")),
        )
        .with_subcommand(
            "text-size",
            JkCommand::new()
                .help("Get or set the dynamic type size")
                .with_argument(
                    JkArgument::new().with_help("size name, e.g. large; omit to read it"),
                ),
        )
        .with_subcommand("reduce-motion", toggle("Get or set Reduce Motion"))
        .with_subcommand(
            "increase-contrast",
            JkCommand::new()
                .help("Set Increase Contrast (the device exposes no getter)")
                .with_argument(JkArgument::new().with_help("on or off").required(true)),
        )
        .with_subcommand(
            "show-borders",
            toggle("Get or set the layout-debug borders overlay"),
        )
        .with_subcommand(
            "reduce-transparency",
            toggle("Get or set Reduce Transparency"),
        )
        .with_flag(JkFlag::new("raw").with_help("Print raw values instead of a friendly summary"))
        .subcommand_required(true)
}

pub async fn main(arguments: &CollectedArguments, provider: Box<dyn IdeviceProvider>) {
    let (sub_name, sub_args) = arguments
        .first_subcommand()
        .expect("no subcommand passed, pass -h for help");
    let mut sub_args = sub_args.clone();

    let proxy = CoreDeviceProxy::connect(&*provider)
        .await
        .unwrap_or_else(|_| panic!("no core device proxy; detail redacted"));
    let rsd_port = proxy.tunnel_info().server_rsd_port;
    let adapter = proxy.create_software_tunnel().unwrap_or_else(|_| panic!("no software tunnel; detail redacted"));
    let mut adapter = adapter.to_async_handle();
    let stream = adapter.connect(rsd_port).await.unwrap_or_else(|_| panic!("no RSD connect; detail redacted"));
    let mut handshake = RsdHandshake::new(stream).await.unwrap_or_else(|_| panic!("operation failed; detail redacted"));

    let mut client = ConfigurationServiceClient::connect_rsd(&mut adapter, &mut handshake)
        .await
        .unwrap_or_else(|_| panic!("no configuration service; detail redacted"));

    match sub_name.as_str() {
        "appearance" => match sub_args.next_argument::<String>().as_deref() {
            None => {
                let style = client
                    .get_user_interface_style()
                    .await
                    .unwrap_or_else(|_| panic!("failed to read the appearance; detail redacted"));
                println!("{}", style.as_str());
            }
            Some("dark") => set_appearance(&mut client, UserInterfaceStyle::Dark).await,
            Some("light") => set_appearance(&mut client, UserInterfaceStyle::Light).await,
            Some(other) => eprintln!("appearance must be 'dark' or 'light', got {other:?}"),
        },
        "opacity" => {
            let opacity: f32 = sub_args.next_argument().expect("opacity required");
            client
                .set_liquid_glass_opacity(opacity)
                .await
                .unwrap_or_else(|_| panic!("failed to set the opacity; detail redacted"));
            println!("opacity set to {opacity}");
        }
        "color-filter" => match sub_args.next_argument::<String>() {
            None => {
                let filter = client
                    .get_color_filter()
                    .await
                    .unwrap_or_else(|_| panic!("failed to read the color filter; detail redacted"));
                println!("{filter:#?}");
            }
            Some(state) => {
                let Some(enabled) = parse_bool(&state) else {
                    eprintln!("expected 'on' or 'off', got {state:?}");
                    return;
                };
                let filter_type = sub_args.next_argument::<String>();
                let intensity = sub_args.next_argument::<f32>();
                client
                    .set_color_filter(enabled, filter_type.as_deref(), intensity)
                    .await
                    .unwrap_or_else(|_| panic!("failed to set the color filter; detail redacted"));
                println!(
                    "color filter {}",
                    if enabled { "enabled" } else { "disabled" }
                );
            }
        },
        "text-size" => match sub_args.next_argument::<String>() {
            None => {
                let size = client
                    .get_device_text_size()
                    .await
                    .unwrap_or_else(|_| panic!("failed to read the text size; detail redacted"));
                println!("{size}");
            }
            Some(size) => {
                client
                    .set_device_text_size(&size)
                    .await
                    .unwrap_or_else(|_| panic!("failed to set the text size; detail redacted"));
                println!("text size set to {size}");
            }
        },
        "reduce-motion" => match sub_args.next_argument::<String>() {
            None => println!(
                "{}",
                client
                    .get_reduce_motion()
                    .await
                    .unwrap_or_else(|_| panic!("failed to read reduce motion; detail redacted"))
            ),
            Some(state) => {
                let Some(enabled) = parse_bool(&state) else {
                    eprintln!("expected 'on' or 'off', got {state:?}");
                    return;
                };
                client
                    .set_reduce_motion(enabled)
                    .await
                    .unwrap_or_else(|_| panic!("failed to set reduce motion; detail redacted"));
                println!("reduce motion {enabled}");
            }
        },
        "increase-contrast" => {
            let state: String = sub_args.next_argument().expect("on or off required");
            let Some(enabled) = parse_bool(&state) else {
                eprintln!("expected 'on' or 'off', got {state:?}");
                return;
            };
            client
                .set_increase_contrast(enabled)
                .await
                .unwrap_or_else(|_| panic!("failed to set increase contrast; detail redacted"));
            println!("increase contrast {enabled}");
        }
        "show-borders" => match sub_args.next_argument::<String>() {
            None => println!(
                "{}",
                client
                    .get_show_borders()
                    .await
                    .unwrap_or_else(|_| panic!("failed to read show borders; detail redacted"))
            ),
            Some(state) => {
                let Some(enabled) = parse_bool(&state) else {
                    eprintln!("expected 'on' or 'off', got {state:?}");
                    return;
                };
                client
                    .set_show_borders(enabled)
                    .await
                    .unwrap_or_else(|_| panic!("failed to set show borders; detail redacted"));
                println!("show borders {enabled}");
            }
        },
        "reduce-transparency" => match sub_args.next_argument::<String>() {
            None => println!(
                "{}",
                client
                    .get_reduce_transparency()
                    .await
                    .unwrap_or_else(|_| panic!("failed to read reduce transparency; detail redacted"))
            ),
            Some(state) => {
                let Some(enabled) = parse_bool(&state) else {
                    eprintln!("expected 'on' or 'off', got {state:?}");
                    return;
                };
                client
                    .set_reduce_transparency(enabled)
                    .await
                    .unwrap_or_else(|_| panic!("failed to set reduce transparency; detail redacted"));
                println!("reduce transparency {enabled}");
            }
        },
        _ => unreachable!(),
    }
}

async fn set_appearance(
    client: &mut ConfigurationServiceClient<Box<dyn idevice::ReadWrite>>,
    style: UserInterfaceStyle,
) {
    client
        .set_user_interface_style(style)
        .await
        .unwrap_or_else(|_| panic!("failed to set the appearance; detail redacted"));
    println!("appearance set to {}", style.as_str());
}

fn parse_bool(value: &str) -> Option<bool> {
    match value {
        "on" | "true" | "1" | "yes" => Some(true),
        "off" | "false" | "0" | "no" => Some(false),
        _ => None,
    }
}
