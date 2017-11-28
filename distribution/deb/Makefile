BASE_NAME=service-world
BIN_DISCOVERY=sw-discover
BIN_PRESENT=sw-present

all:
	@echo 'Example: SRC_DIR=../../ BIN_DIR=../../target/release ARCH=amd64 VERSION=0.1.0-alpha-3 TAG=$VERSION DIST=trusty make package'

package: $(BASE_NAME)-$(VERSION)-$(DIST)-$(ARCH).deb

$(BASE_NAME)-$(VERSION)-$(DIST)-$(ARCH).deb: $(BASE_NAME)/DEBIAN/control $(BASE_NAME)/DEBIAN/conffiles $(BASE_NAME)/DEBIAN/postinst $(BASE_NAME)/DEBIAN/prerm $(BASE_NAME)/usr/bin/$(BIN_DISCOVERY) $(BASE_NAME)/usr/sbin/$(BIN_PRESENT) $(BASE_NAME)/var/lib/$(BASE_NAME)/templates/index.html.hbs $(BASE_NAME)/var/lib/$(BASE_NAME)/templates/services.html.hbs
	dpkg-deb -b $(BASE_NAME) $@
	dpkg-deb -I $@

clean:
	-rm -fR $(BASE_NAME)/DEBIAN
	-rm -fR $(BASE_NAME)/usr
	-rm $(BASE_NAME)-$(VERSION)-$(DIST)-$(ARCH).deb

$(BASE_NAME)/DEBIAN/control: templates/DEBIAN/control $(BASE_NAME)/usr/bin/$(BIN_DISCOVERY) $(BASE_NAME)/usr/sbin/$(BIN_PRESENT) $(BASE_NAME)/DEBIAN
	SIZE=`du -cs $(BASE_NAME)/ | tail -1 | awk '{ print $$1}'`; SANE_VERSION=`echo $$VERSION | sed 's/^[a-z]*//'`; sed "s/@@PACKAGE_NAME@@/$(BASE_NAME)/; s/@@VERSION@@/$${SANE_VERSION}/; s/@@ARCH@@/$(ARCH)/; s/@@SIZE@@/$${SIZE}/" $< > $@

$(BASE_NAME)/usr/bin/$(BIN_DISCOVERY): $(BIN_DIR)/$(BIN_DISCOVERY) $(BASE_NAME)/usr/bin
	cp $< $@

$(BASE_NAME)/usr/sbin/$(BIN_PRESENT): $(BIN_DIR)/$(BIN_PRESENT) $(BASE_NAME)/usr/sbin
	cp $< $@

$(BASE_NAME)/var/lib/$(BASE_NAME)/templates/index.html.hbs: $(SRC_DIR)/examples/templates/index.html.hbs $(BASE_NAME)/var/lib/$(BASE_NAME)/templates
	cp $< $@

$(BASE_NAME)/var/lib/$(BASE_NAME)/templates/services.html.hbs: $(SRC_DIR)/examples/templates/services.html.hbs $(BASE_NAME)/var/lib/$(BASE_NAME)/templates
	cp $< $@

$(BASE_NAME)/DEBIAN/postinst: templates/DEBIAN/postinst
	cp $< $@
	chmod 755 $@

$(BASE_NAME)/DEBIAN/prerm: templates/DEBIAN/prerm
	cp $< $@
	chmod 755 $@

$(BASE_NAME)/DEBIAN/conffiles: templates/DEBIAN/conffiles
	cp $< $@

$(BASE_NAME)/etc/$(BASE_NAME):
	mkdir -p $@

$(BASE_NAME)/usr/bin:
	mkdir -p $@

$(BASE_NAME)/usr/sbin:
	mkdir -p $@

$(BASE_NAME)/var/lib/$(BASE_NAME)/templates:
	mkdir -p $@

$(BASE_NAME)/DEBIAN:
	mkdir -p $@

