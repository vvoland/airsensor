OUT=build
OBJ=obj

.DEFAULT_GOAL=upload

$(OUT)/%.js: %.ts
	tsc --lib es2015,dom $< --outFile $@

$(OUT)/%.html: %.html
	cp $< $@

$(OUT)/%.css: %.css
	cp $< $@

$(OUT)/script.bundle.js: $(OUT)/sensor-card.js $(OUT)/index.html $(OUT)/style.css
	cat $(OUT)/*.js >$(OUT)/script.bundle.js

.PHONY: clean
clean:
	rm -r $(OUT)

ui: $(OUT)/script.bundle.js $(OUT)/index.html $(OUT)/style.css

.PHONY: upload
upload: ui
	rsync --filter=':- .gitignore' -avp ./build/ pi@raspberrypi.lan:~/app/
