#!/usr/bin/env bash

VERSION=`yq .package.version Cargo.toml`
DOCKER_REPO="ghcr.io/translatorsri"

MAJOR_VERSION=`echo ${VERSION:0:2} | sed 's:\.::g'`
MINOR_VERSION=`echo ${VERSION:2:2} | sed 's:\.::g'`
PATCH_VERSION=`echo ${VERSION: -2} | sed 's:\.::g'`

RELEASE_VERSION=$MAJOR_VERSION.$MINOR_VERSION.$(( PATCH_VERSION + 1 ))

echo "RELEASE_VERSION: $RELEASE_VERSION"

sed -i -e "s|version = \"$VERSION\"|version = \"$RELEASE_VERSION\"|g" Cargo.toml
sed -i -e "s|version: \"$VERSION\"|version: \"$RELEASE_VERSION\"|g" helm/Chart.yaml
sed -i -e "s|tag: \"$VERSION\"|tag: \"$RELEASE_VERSION\"|g" helm/values.yaml

docker build -t $DOCKER_REPO/cqs:$RELEASE_VERSION .
docker push $DOCKER_REPO/cqs:$RELEASE_VERSION

helm package -d docs helm
helm repo index docs --merge docs/index.yaml --url https://translatorsri.github.io/CQS

