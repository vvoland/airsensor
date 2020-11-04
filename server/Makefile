APP=./target/armv7-unknown-linux-gnueabihf/debug/server
STRIP=/usr/bin/arm-linux-gnueabihf-strip

server:
	cargo build --target armv7-unknown-linux-gnueabihf
	$(STRIP)  $(APP)

upload:
	rsync -avPz target/armv7-unknown-linux-gnueabihf/debug/server pi@raspberrypi.local:~/weather

