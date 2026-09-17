# Core + Utils are Rust C-ABI drop-ins (rust/cjson, rust/cjson_utils).
# cJSON_Utils.c remains in-tree but is not built by this Makefile.
CJSON_RUST_DIR = rust/cjson
CJSON_RUST_TARGET = $(CURDIR)/build-cargo-cjson
CJSON_RUST_LIB = $(CJSON_RUST_TARGET)/release/libcjson.a
UTILS_RUST_DIR = rust/cjson_utils
UTILS_RUST_TARGET = $(CURDIR)/build-cargo-cjson-utils
UTILS_RUST_LIB = $(UTILS_RUST_TARGET)/release/libcjson_utils.a
CJSON_LIBNAME = libcjson
UTILS_LIBNAME = libcjson_utils
CJSON_TEST = cJSON_test

CJSON_TEST_SRC = test.c

LDLIBS = -lm

LIBVERSION = 1.7.19
CJSON_SOVERSION = 1
UTILS_SOVERSION = 1

CJSON_SO_LDFLAG=-Wl,-soname=$(CJSON_LIBNAME).so.$(CJSON_SOVERSION)
UTILS_SO_LDFLAG=-Wl,-soname=$(UTILS_LIBNAME).so.$(UTILS_SOVERSION)

PREFIX ?= /usr/local
INCLUDE_PATH ?= include/cjson
LIBRARY_PATH ?= lib

INSTALL_INCLUDE_PATH = $(DESTDIR)$(PREFIX)/$(INCLUDE_PATH)
INSTALL_LIBRARY_PATH = $(DESTDIR)$(PREFIX)/$(LIBRARY_PATH)

INSTALL ?= cp -a

CC = gcc -std=c89

# validate gcc version for use fstack-protector-strong
MIN_GCC_VERSION = "4.9"
GCC_VERSION := "`$(CC) -dumpversion`"
IS_GCC_ABOVE_MIN_VERSION := $(shell expr "$(GCC_VERSION)" ">=" "$(MIN_GCC_VERSION)")
ifeq "$(IS_GCC_ABOVE_MIN_VERSION)" "1"
    CFLAGS += -fstack-protector-strong
else
    CFLAGS += -fstack-protector
endif

PIC_FLAGS = -fPIC
R_CFLAGS = $(PIC_FLAGS) -pedantic -Wall -Werror -Wstrict-prototypes -Wwrite-strings -Wshadow -Winit-self -Wcast-align -Wformat=2 -Wmissing-prototypes -Wstrict-overflow=2 -Wcast-qual -Wc++-compat -Wundef -Wswitch-default -Wconversion $(CFLAGS)

uname := $(shell sh -c 'uname -s 2>/dev/null || echo false')

#library file extensions
SHARED = so
STATIC = a

## create dynamic (shared) library on Darwin (base OS for MacOSX and IOS)
ifeq (Darwin, $(uname))
	SHARED = dylib
	CJSON_SO_LDFLAG = ""
	UTILS_SO_LDFLAG = ""
endif

#cJSON library names
CJSON_SHARED = $(CJSON_LIBNAME).$(SHARED)
CJSON_SHARED_VERSION = $(CJSON_LIBNAME).$(SHARED).$(LIBVERSION)
CJSON_SHARED_SO = $(CJSON_LIBNAME).$(SHARED).$(CJSON_SOVERSION)
CJSON_STATIC = $(CJSON_LIBNAME).$(STATIC)

#cJSON_Utils library names
UTILS_SHARED = $(UTILS_LIBNAME).$(SHARED)
UTILS_SHARED_VERSION = $(UTILS_LIBNAME).$(SHARED).$(LIBVERSION)
UTILS_SHARED_SO = $(UTILS_LIBNAME).$(SHARED).$(UTILS_SOVERSION)
UTILS_STATIC = $(UTILS_LIBNAME).$(STATIC)

SHARED_CMD = $(CC) -shared -o

.PHONY: all shared static tests clean install

all: shared static tests

shared: $(CJSON_SHARED) $(UTILS_SHARED)

static: $(CJSON_STATIC) $(UTILS_STATIC)

tests: $(CJSON_TEST)

test: tests
	./$(CJSON_TEST)

.c.o:
	$(CC) -c $(R_CFLAGS) $<

#tests
#cJSON
$(CJSON_TEST): $(CJSON_TEST_SRC) cJSON.h $(CJSON_RUST_LIB)
	$(CC) $(R_CFLAGS) $(CJSON_TEST_SRC) -o $@ -Wl,--whole-archive $(CJSON_RUST_LIB) -Wl,--no-whole-archive $(LDLIBS) -lpthread -ldl -I.

#static libraries
#cJSON (Rust C-ABI archive)
$(CJSON_RUST_LIB):
	CARGO_TARGET_DIR=$(CJSON_RUST_TARGET) cargo build --release --manifest-path $(CJSON_RUST_DIR)/Cargo.toml

$(UTILS_RUST_LIB): $(CJSON_RUST_LIB)
	CARGO_TARGET_DIR=$(UTILS_RUST_TARGET) cargo build --release --manifest-path $(UTILS_RUST_DIR)/Cargo.toml

$(CJSON_STATIC): $(CJSON_RUST_LIB)
	cp $(CJSON_RUST_LIB) $@

# Utils staticlib embeds core via path dep; do not also archive libcjson.a here.
$(UTILS_STATIC): $(UTILS_RUST_LIB)
	cp $(UTILS_RUST_LIB) $@

#shared libraries .so.1.0.0
#cJSON
$(CJSON_SHARED_VERSION): $(CJSON_RUST_LIB)
	$(CC) -shared -o $@ -Wl,--whole-archive $(CJSON_RUST_LIB) -Wl,--no-whole-archive $(CJSON_SO_LDFLAG) $(LDFLAGS) $(LDLIBS) -lpthread -ldl

# Utils shared: whole-archive Rust utils (embeds core). Do not also link libcjson.
$(UTILS_SHARED_VERSION): $(UTILS_RUST_LIB)
	$(CC) -shared -o $@ -Wl,--whole-archive $(UTILS_RUST_LIB) -Wl,--no-whole-archive $(UTILS_SO_LDFLAG) $(LDFLAGS) $(LDLIBS) -lpthread -ldl

#links .so -> .so.1 -> .so.1.0.0
#cJSON
$(CJSON_SHARED_SO): $(CJSON_SHARED_VERSION)
	ln -sf $(CJSON_SHARED_VERSION) $(CJSON_SHARED_SO)
$(CJSON_SHARED): $(CJSON_SHARED_SO)
	ln -sf $(CJSON_SHARED_SO) $(CJSON_SHARED)
#cJSON_Utils
$(UTILS_SHARED_SO): $(UTILS_SHARED_VERSION)
	ln -sf $(UTILS_SHARED_VERSION) $(UTILS_SHARED_SO)
$(UTILS_SHARED): $(UTILS_SHARED_SO)
	ln -sf $(UTILS_SHARED_SO) $(UTILS_SHARED)

#install
#cJSON
install-cjson:
	mkdir -p $(INSTALL_LIBRARY_PATH) $(INSTALL_INCLUDE_PATH)
	$(INSTALL) cJSON.h $(INSTALL_INCLUDE_PATH)
	$(INSTALL) $(CJSON_SHARED) $(CJSON_SHARED_SO) $(CJSON_SHARED_VERSION) $(INSTALL_LIBRARY_PATH)
#cJSON_Utils
install-utils: install-cjson
	$(INSTALL) cJSON_Utils.h $(INSTALL_INCLUDE_PATH)
	$(INSTALL) $(UTILS_SHARED) $(UTILS_SHARED_SO) $(UTILS_SHARED_VERSION) $(INSTALL_LIBRARY_PATH)

install: install-cjson install-utils

#uninstall
#cJSON
uninstall-cjson: uninstall-utils
	$(RM) $(INSTALL_LIBRARY_PATH)/$(CJSON_SHARED)
	$(RM) $(INSTALL_LIBRARY_PATH)/$(CJSON_SHARED_VERSION)
	$(RM) $(INSTALL_LIBRARY_PATH)/$(CJSON_SHARED_SO)
	$(RM) $(INSTALL_INCLUDE_PATH)/cJSON.h
	
#cJSON_Utils
uninstall-utils:
	$(RM) $(INSTALL_LIBRARY_PATH)/$(UTILS_SHARED)
	$(RM) $(INSTALL_LIBRARY_PATH)/$(UTILS_SHARED_VERSION)
	$(RM) $(INSTALL_LIBRARY_PATH)/$(UTILS_SHARED_SO)
	$(RM) $(INSTALL_INCLUDE_PATH)/cJSON_Utils.h

remove-dir:
	$(if $(wildcard $(INSTALL_LIBRARY_PATH)/*.*),,rmdir $(INSTALL_LIBRARY_PATH))
	$(if $(wildcard $(INSTALL_INCLUDE_PATH)/*.*),,rmdir $(INSTALL_INCLUDE_PATH))

uninstall: uninstall-utils uninstall-cjson remove-dir

clean:
	$(RM) $(CJSON_SHARED) $(CJSON_SHARED_VERSION) $(CJSON_SHARED_SO) $(CJSON_STATIC)
	$(RM) $(UTILS_SHARED) $(UTILS_SHARED_VERSION) $(UTILS_SHARED_SO) $(UTILS_STATIC)
	$(RM) $(CJSON_TEST)
	$(RM) -r $(CJSON_RUST_TARGET) $(UTILS_RUST_TARGET)
