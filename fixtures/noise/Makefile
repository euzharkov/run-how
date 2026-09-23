SPHINXBUILD ?= sphinx-build
BUILDDIR    = _build
BINS        := a b

.PHONY: docs clean install help

## Build the HTML documentation
docs:
	$(SPHINXBUILD) -b html docs $(BUILDDIR)/html

clean:
	$(RM) -r $(BUILDDIR)/*

install:
	$(foreach c, $(BINS), install $(c) /usr/local/bin/$(c);)

help:
	@echo "targets: docs clean install"
