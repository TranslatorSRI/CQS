#!/usr/bin/env bash

VERSION=`yq .package.version Cargo.toml`
DOCKER_REPO="ghcr.io/translatorsri"

MAJOR_VERSION=`echo ${VERSION:0:2} | sed 's:\.::g'`
MINOR_VERSION=`echo ${VERSION:2:2} | sed 's:\.::g'`
PATCH_VERSION=`echo ${VERSION: -2} | sed 's:\.::g'`

RELEASE_VERSION=$MAJOR_VERSION.$MINOR_VERSION.$(( PATCH_VERSION + 1 ))
echo "RELEASE_VERSION: $RELEASE_VERSION"

MAJOR_MINOR_VERSION="$MAJOR_VERSION.$MINOR_VERSION"
echo "MAJOR_MINOR_VERSION: $MAJOR_MINOR_VERSION"

sed -i -e "s|version = \"$VERSION\"|version = \"$RELEASE_VERSION\"|g" Cargo.toml
yq -i ".version = \"$RELEASE_VERSION\"" helm/Chart.yaml
yq -i ".image.tag = \"$RELEASE_VERSION\"" helm/values.yaml
#yq -i ".ingress.major_minor_version = \"$MAJOR_MINOR_VERSION\"" helm/values.yaml

docker build -t $DOCKER_REPO/cqs:$RELEASE_VERSION .
docker push $DOCKER_REPO/cqs:$RELEASE_VERSION

helm package -d docs helm
helm repo index docs --merge docs/index.yaml --url https://translatorsri.github.io/CQS

git commit -m "incrementing version" Cargo.toml docs/index.yaml helm/Chart.yaml helm/values.yaml
git add docs/cqs-$RELEASE_VERSION.tgz
git commit -m "initial commit" docs/cqs-$RELEASE_VERSION.tgz
