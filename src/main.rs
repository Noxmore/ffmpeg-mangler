#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

keystone::flat! {
	prelude;
}

fn main()
{
	let native_options = NativeOptions
	{
		..Default::default()
	};
	run_native("Media Mangler", native_options, Box::new(|cc| Box::new(ManglerApp::new(cc)))).unwrap();
}

#[derive(SmartDefault)]
struct ManglerApp
{
	path: String,

	#[default(30)]
	fps: u16,
	#[default(640)]
	scale: u16,
	#[default(50)]
	noise_amount: u8,
	#[default(250)]
	video_bitrate: u16,
	#[default(1)]
	audio_bitrate: u16,
}

impl ManglerApp
{
	fn new(cc: &CreationContext<'_>) -> Self
	{
		let mut visuals = Visuals::dark();
		
		//let rounding = Rounding::none();
		//visuals.widgets.active.rounding = rounding;
		//visuals.widgets.inactive.rounding = rounding;
		//visuals.widgets.hovered.rounding = rounding;
		//visuals.widgets.noninteractive.rounding = rounding;
		visuals.widgets.noninteractive.fg_stroke = Stroke::new(1., Color32::WHITE);
		
		//let mut fonts = FontDefinitions::default();
		//for (_name, data) in &mut fonts.font_data { data.tweak.scale *= 1.5; }
		
		//cc.egui_ctx.set_fonts(fonts);
		cc.egui_ctx.set_visuals(visuals);
		cc.egui_ctx.set_pixels_per_point(1.2);

		run_preview_thread();

		Self::default()
	}
}

impl App for ManglerApp
{
	fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame)
	{
		SidePanel::left("settings").show(ctx, |ui|
		{
			ui.horizontal(|ui| {
				ui.label("File");
				ui.text_edit_singleline(&mut self.path);
				if ui.button("...").clicked() {
					if let Some(path) = rfd::FileDialog::new()
						.set_title("Select Media")
						.add_filter("Audio/Video Files", &["mp4", "mkv", "mov", "avi", "wmv", "webm", "wav", "aiff", "mp3", "ogg", "aac", "flac"])
						.pick_file()
					{
						self.path = path.display().to_string();
					}
				}
			});

			let spacing = 20.;

			ui.add_space(spacing);
			ui.label("FPS");
			Slider::new(&mut self.fps, 1..=60).ui(ui);


			ui.add_space(spacing);
			ui.label("Scale");
			Slider::new(&mut self.scale, 1..=1920)
				.custom_formatter(|n, _| format!("{n}x{}", height_from_width(n as f32)))
				.ui(ui)
				.on_hover_text("(Based on 16/9 aspect ratio, may not be accurate for all resolutions)");
			ui.horizontal(|ui| {
				if ui.button("144p").clicked() { self.scale = width_from_height(144.) as u16 }
				if ui.button("240p").clicked() { self.scale = width_from_height(240.) as u16 }
				if ui.button("360p").clicked() { self.scale = width_from_height(360.) as u16 }
				if ui.button("480p").clicked() { self.scale = width_from_height(480.) as u16 }
				if ui.button("720p").clicked() { self.scale = width_from_height(720.) as u16 }
			});


			ui.add_space(spacing);
			ui.label("Noise Amount");
			Slider::new(&mut self.noise_amount, 0..=100).ui(ui);

			ui.add_space(spacing);
			ui.label("Video Bitrate (kbps)");
			DragValue::new(&mut self.video_bitrate).ui(ui);

			ui.add_space(spacing);
			ui.label("Audio Bitrate (kbps)");
			DragValue::new(&mut self.audio_bitrate).ui(ui);


			ui.with_layout(Layout::right_to_left(Align::BOTTOM), |ui| {
				if ui.add_sized([70., 30.], Button::new("Render")).clicked() {

				}
			});
		});

		CentralPanel::default().show(ctx, |ui|
		{
			ui.heading("(no file)");
		});
	}
}

fn height_from_width(width: impl Into<f32>) -> u16 {
	(width.into() / (16./9.)).round() as u16
}
fn width_from_height(height: impl Into<f32>) -> u16 {
	(height.into() * (16./9.)).round() as u16
}

#[derive(Default)]
pub enum PreviewState {
	#[default]
	NoFile,
	Loading(String),
	Loaded(ColorImage),
	Failed,
	InternalError(io::Error),
}

pub static CURRENT_PREVIEW_STATE: Lazy<Mutex<PreviewState>> = Lazy::new(Default::default);

fn run_preview_thread() {
	thread::spawn(|| {
		loop {
			let mut state = CURRENT_PREVIEW_STATE.lock().unwrap();
			if let Err(err) = || -> io::Result<()> {
				if let PreviewState::Loading(path) = &*state {
					let stdout = Command::new("ffprobe")
						// .args(["-i", path.as_str(), "-show_entries", "format=duration", "-v", "quiet", "-of", "csv=p=0"])
						.args(shlex::split(&format!("-i {path} -show_entries format=duration -v quiet -of csv=\"p=0\"")).unwrap())
						.output()?.stdout;

					let len: f32 = std::str::from_utf8(&stdout).map_err(invalid_data)?.parse().map_err(invalid_data)?;

					Command::new("ffmpeg")
						.args(["-accurate_seek", "-ss", (fastrand::f32() * len).to_string().as_str(), "-i", "mangled.mp4", "-r", "1/1", "-frames:v", "1", "mangled.jpg"])
						.output()?;
				}

				Ok(())
			}() {
				*state = PreviewState::InternalError(err);
			}
			
			thread::sleep(Duration::from_secs_f32(0.01));
		}
	});
}

fn invalid_data(err: impl std::error::Error + Send + Sync + 'static) -> io::Error {
	io::Error::new(io::ErrorKind::InvalidData, err)
}