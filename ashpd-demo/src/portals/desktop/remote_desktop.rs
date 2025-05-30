use std::sync::Arc;

use adw::{prelude::*, subclass::prelude::*};
use ashpd::{
    desktop::{
        remote_desktop::{DeviceType, RemoteDesktop},
        screencast::{CursorMode, Screencast, SourceType, Stream},
        PersistMode, Session,
    },
    enumflags2::BitFlags,
    WindowIdentifier,
};
use futures_util::lock::Mutex;
use gtk::glib::{self, clone};

use crate::widgets::{PortalPage, PortalPageExt, PortalPageImpl};

mod imp {
    use super::*;
    use crate::portals::desktop::screencast::available_types;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(resource = "/com/belmoussaoui/ashpd/demo/remote_desktop.ui")]
    pub struct RemoteDesktopPage {
        #[template_child]
        pub response_group: TemplateChild<adw::PreferencesGroup>,
        pub session: Arc<Mutex<Option<Session<'static, RemoteDesktop<'static>>>>>,
        #[template_child]
        pub screencast_switch: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub multiple_switch: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub keyboard_check: TemplateChild<gtk::CheckButton>,
        #[template_child]
        pub pointer_check: TemplateChild<gtk::CheckButton>,
        #[template_child]
        pub touchscreen_check: TemplateChild<gtk::CheckButton>,
        #[template_child]
        pub monitor_check: TemplateChild<gtk::CheckButton>,
        #[template_child]
        pub window_check: TemplateChild<gtk::CheckButton>,
        #[template_child]
        pub virtual_check: TemplateChild<gtk::CheckButton>,
        #[template_child]
        pub cursor_mode_combo: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub persist_mode_combo: TemplateChild<adw::ComboRow>,
        pub session_token: Arc<Mutex<Option<String>>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for RemoteDesktopPage {
        const NAME: &'static str = "RemoteDesktopPage";
        type Type = super::RemoteDesktopPage;
        type ParentType = PortalPage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();

            klass.install_action_async("remote_desktop.start", None, |page, _, _| async move {
                page.start_session().await;
            });
            klass.install_action_async("remote_desktop.stop", None, |page, _, _| async move {
                page.stop_session().await;
                page.info("Remote desktop session stopped");
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }
    impl ObjectImpl for RemoteDesktopPage {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().action_set_enabled("remote_desktop.stop", false);
        }
    }
    impl WidgetImpl for RemoteDesktopPage {
        fn map(&self) {
            let widget = self.obj();
            glib::spawn_future_local(clone!(
                #[weak]
                widget,
                async move {
                    let imp = widget.imp();
                    if let Ok((cursor_modes, source_types)) = available_types().await {
                        imp.virtual_check
                            .set_sensitive(source_types.contains(SourceType::Virtual));
                        imp.monitor_check
                            .set_sensitive(source_types.contains(SourceType::Monitor));
                        imp.window_check
                            .set_sensitive(source_types.contains(SourceType::Window));
                        let model = gtk::StringList::default();
                        if cursor_modes.contains(CursorMode::Hidden) {
                            model.append("Hidden");
                        }
                        if cursor_modes.contains(CursorMode::Metadata) {
                            model.append("Metadata");
                        }
                        if cursor_modes.contains(CursorMode::Embedded) {
                            model.append("Embedded");
                        }
                        imp.cursor_mode_combo.set_model(Some(&model));
                    }

                    if let Ok(devices) = available_devices().await {
                        imp.touchscreen_check
                            .set_sensitive(devices.contains(DeviceType::Touchscreen));
                        imp.pointer_check
                            .set_sensitive(devices.contains(DeviceType::Pointer));
                        imp.keyboard_check
                            .set_sensitive(devices.contains(DeviceType::Keyboard));
                    }
                }
            ));
            self.parent_map();
        }
    }
    impl BinImpl for RemoteDesktopPage {}
    impl PortalPageImpl for RemoteDesktopPage {}
}

glib::wrapper! {
    pub struct RemoteDesktopPage(ObjectSubclass<imp::RemoteDesktopPage>)
        @extends gtk::Widget, adw::Bin, PortalPage;
}

impl RemoteDesktopPage {
    /// Returns the selected DeviceType
    fn selected_devices(&self) -> BitFlags<DeviceType> {
        let imp = self.imp();

        let mut devices: BitFlags<DeviceType> = BitFlags::empty();
        if imp.keyboard_check.is_active() {
            devices.insert(DeviceType::Keyboard);
        }
        if imp.pointer_check.is_active() {
            devices.insert(DeviceType::Pointer);
        }
        if imp.touchscreen_check.is_active() {
            devices.insert(DeviceType::Touchscreen);
        }
        devices
    }

    /// Returns the selected SourceType
    fn selected_sources(&self) -> BitFlags<SourceType> {
        let imp = self.imp();
        let mut sources: BitFlags<SourceType> = BitFlags::empty();
        if imp.monitor_check.is_active() {
            sources.insert(SourceType::Monitor);
        }
        if imp.window_check.is_active() {
            sources.insert(SourceType::Window);
        }
        sources
    }

    fn selected_persist_mode(&self) -> PersistMode {
        match self.imp().persist_mode_combo.selected() {
            0 => PersistMode::DoNot,
            1 => PersistMode::Application,
            2 => PersistMode::ExplicitlyRevoked,
            _ => unreachable!(),
        }
    }

    /// Returns the selected CursorMode
    fn selected_cursor_mode(&self) -> CursorMode {
        match self
            .imp()
            .cursor_mode_combo
            .selected_item()
            .and_downcast::<gtk::StringObject>()
            .unwrap()
            .string()
            .as_ref()
        {
            "Hidden" => CursorMode::Hidden,
            "Embedded" => CursorMode::Embedded,
            "Metadata" => CursorMode::Metadata,
            _ => unreachable!(),
        }
    }

    async fn start_session(&self) {
        let imp = self.imp();

        self.action_set_enabled("remote_desktop.start", false);
        self.action_set_enabled("remote_desktop.stop", true);

        match self.remote().await {
            Ok((_selected_devices, _streams, session)) => {
                imp.response_group.set_visible(true);
                imp.session.lock().await.replace(session);
                self.action_set_enabled("remote_desktop.start", false);
                self.action_set_enabled("remote_desktop.stop", true);
                self.success("Remote desktop session started successfully");
            }
            Err(err) => {
                tracing::error!("Failed to start remote desktop session: {err}");
                self.error("Failed to start a remote desktop session");
                self.stop_session().await;
            }
        };
    }

    async fn stop_session(&self) {
        self.action_set_enabled("remote_desktop.start", true);
        self.action_set_enabled("remote_desktop.stop", false);

        let imp = self.imp();
        if let Some(session) = imp.session.lock().await.take() {
            let _ = session.close().await;
        }
        imp.response_group.set_visible(false);
    }

    async fn remote(
        &self,
    ) -> ashpd::Result<(
        BitFlags<DeviceType>,
        Vec<Stream>,
        Session<'static, RemoteDesktop<'static>>,
    )> {
        let imp = self.imp();
        let root = self.native().unwrap();
        let identifier = WindowIdentifier::from_native(&root).await;
        let is_screencast = imp.screencast_switch.get().is_active();
        let multiple_sources = imp.multiple_switch.is_active();
        let cursor_mode = self.selected_cursor_mode();
        let sources = self.selected_sources();
        let devices = self.selected_devices();
        let persist_mode = self.selected_persist_mode();
        let mut token = imp.session_token.lock().await;

        let proxy = RemoteDesktop::new().await?;
        let session = proxy.create_session().await?;
        if is_screencast {
            let screencast_proxy = Screencast::new().await?;
            screencast_proxy
                .select_sources(
                    &session,
                    cursor_mode,
                    sources,
                    multiple_sources,
                    None,
                    PersistMode::default(),
                )
                .await?;
        }
        proxy
            .select_devices(&session, devices, token.as_deref(), persist_mode)
            .await?;

        self.info("Starting a remote desktop session");
        let response = proxy
            .start(&session, identifier.as_ref())
            .await?
            .response()?;
        if let Some(t) = response.restore_token() {
            token.replace(t.to_owned());
        }

        Ok((
            response.devices(),
            response.streams().unwrap_or_default().to_owned(),
            session,
        ))
    }
}

pub async fn available_devices() -> ashpd::Result<BitFlags<DeviceType>> {
    let proxy = RemoteDesktop::new().await?;
    proxy.available_device_types().await
}
