test:
	cargo test --locked -- --test-threads=1

lint:
	cargo clippy --locked --all-targets --all-features -- -D warnings -A clippy::derivable_impls -A clippy::large-enum-variant

vrt:
	node vrt/index.js && reg-cli vrt/screenshot/actual vrt/screenshot/expected vrt/screenshot/diff -R vrt/report.html -I

vrt-update:
	node vrt/index.js && reg-cli vrt/screenshot/actual vrt/screenshot/expected vrt/screenshot/diff -R vrt/report.html -U