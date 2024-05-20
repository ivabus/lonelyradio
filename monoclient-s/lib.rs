mod main;

#[no_mangle]
fn android_main(app: slint::android::AndroidApp) {
	slint::android::init(app).unwrap();
	main::main()
}
