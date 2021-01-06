OUT=build
OBJ=obj
HTML_DIR=html
CSS_DIR=css

.DEFAULT_GOAL=upload

$(OUT)/%.js: %.ts
	tsc --allowJs --lib es2015,dom $< --outFile $@

$(OUT)/%.html: $(HTML_DIR)/%.html
	cp $< $@

$(OUT)/%.css: $(CSS_DIR)/%.css
	cp $< $@

$(OUT)/script.bundle.js: typescript-compile
	webpack --entry ./$(OBJ)/index.js --output $(OUT)/script.bundle.js

.PHONY: typescript-compile
typescript-compile:
	tsc --target ES5 --module commonjs --lib es2015,dom --outDir $(OBJ) src/*.ts

.PHONY: clean
clean:
	rm -r $(OUT) || true
	rm -r $(OBJ) || true

ui: $(OUT)/script.bundle.js $(OUT)/index.html $(OUT)/style.css

.PHONY: upload
upload: ui
	rsync --filter=':- .gitignore' -avp ./build/ pi@raspberrypi.lan:~/app/
