#!/bin/bash -e

export CC_arm_linux_androideabi=arm-linux-androideabi-clang

cargo build --target arm-linux-androideabi

cp target/arm-linux-androideabi/debug/mullvad-daemon mullvad-android/app/src/main/assets/

cd mullvad-android
./gradlew installDebug
cd -
