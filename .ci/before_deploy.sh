# This script takes care of building your crate and packaging it for release

set -ex

main() {
    local src=$(pwd) \
          stage=

    case $TRAVIS_OS_NAME in
        linux)
            stage=$(mktemp -d)
            ;;
        osx)
            stage=$(mktemp -d -t tmp)
            ;;
    esac

    test -f Cargo.lock || cargo generate-lockfile

    cross rustc --bin sw-present --target $TARGET --release --features bin-present -- -C lto
    cross rustc --bin sw-discover --target $TARGET --release --features bin-discover -- -C lto

    cp target/$TARGET/release/sw-present $stage/
    cp target/$TARGET/release/sw-discover $stage/

    cd $stage
    tar czf $src/$CRATE_NAME-$TRAVIS_TAG-$TARGET.tar.gz *

    cd $src/distribution/deb
    SRC_DIR=$src BIN_DIR=$src/target/$TARGET/release ARCH=$DEPLOY_ARCH VERSION=$TRAVIS_TAG TAG=$TRAVIS_TAG DIST=trusty make package

    cd $src
    rm -rf $stage
}

main
