#
# Download schemas from DMTF / SNIA sites and then prepare
# build and test everything.
#

pwd := $(shell pwd)

redfish-bundle-version = 2025.2
redfish-bundle-sha256sum = 4ca0400088cfff6eb84851966b1a7fa86181898de0dbbaddfdaba7f8bba8c004
swordfish-bundle-version = v1.2.8
swordfish-bundle-sha256sum = 87a0dd6c7e9a831a519e105b75ba8759ca85314cf92fd782cfd9ce6637f863aa

schema-dir = schemas
schema-dir-dep = $(schema-dir)/.dep
schema-dir-redfish = $(schema-dir)/redfish-csdl
schema-dir-swordfish = $(schema-dir)/swordfish-csdl

redfish-schemas-dep = schemas/.dep-redfish
swordfish-schemas-dep = schemas/.dep-swordfish

all: $(redfish-schemas-dep) $(swordfish-schemas-dep) 
	cargo build
	cargo test -- --no-capture
	cargo clippy
	cargo doc

clean:
	rm -rf $(schema-dir)
	rm -rf target

$(redfish-schemas-dep): $(schema-dir-dep)
	curl -vfL "https://www.dmtf.org/sites/default/files/standards/documents/DSP8010_$(redfish-bundle-version).zip" > $(schema-dir)/redfish-bundle.zip
	printf "%s  %s" $(redfish-bundle-sha256sum) $(schema-dir)/redfish-bundle.zip | shasum -a 256 -c -
	unzip -j -o $(schema-dir)/redfish-bundle.zip "DSP8010_$(redfish-bundle-version)/csdl/*" -d $(schema-dir-redfish)
	touch $@

$(swordfish-schemas-dep): $(schema-dir-dep)
	curl -vfL "https://www.snia.org/sites/default/files/technical-work/swordfish/release/$(swordfish-bundle-version)/zip/Swordfish_$(swordfish-bundle-version).zip" > $(schema-dir)/swordfish-bundle.zip
	printf "%s  %s" $(swordfish-bundle-sha256sum) $(schema-dir)/swordfish-bundle.zip | shasum -a 256 -c -
	unzip -p $(schema-dir)/swordfish-bundle.zip "Swordfish_$(swordfish-bundle-version)_Schema.zip" > $(schema-dir)/swordfish-schema.zip
	unzip -j -o $(schema-dir)/swordfish-schema.zip "csdl-schema/*" -d $(schema-dir-swordfish)
	touch $@

$(schema-dir-dep):
	mkdir $(schema-dir)
	touch $@


