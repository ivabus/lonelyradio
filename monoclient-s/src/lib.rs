mod app;
#[cfg(target_os = "andoid")]
#[no_mangle]
fn android_main(app: slint::android::AndroidApp) {
	slint::android::init(app).unwrap();
	app::_main();
}
