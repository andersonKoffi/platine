#
# Builds the Platine Rust engine (native/dj_api) with cargokit and links it
# statically into the app. Generated from the flutter_rust_bridge 2.13.0
# cargokit template, adapted to a crate living outside the Flutter app.
#
Pod::Spec.new do |s|
  s.name             = 'dj_api'
  s.version          = '0.0.1'
  s.summary          = 'Platine Rust engine (dj_api), built by cargokit.'
  s.description      = <<-DESC
Builds the Platine Rust engine (native/dj_api) with cargokit.
                       DESC
  s.homepage         = 'https://github.com/andersonKoffi/platine'
  s.license          = { :type => 'MIT' }
  s.author           = { 'Sophos Studios' => 'dev@sophosstudios.invalid' }

  s.source           = { :path => '.' }
  s.source_files     = 'Classes/**/*'
  s.dependency 'FlutterMacOS'
  # System frameworks used by the Rust audio stack (cpal on CoreAudio).
  s.frameworks = 'AudioToolbox', 'AudioUnit', 'CoreAudio', 'CoreFoundation', 'Foundation'

  s.platform = :osx, '10.15'
  s.swift_version = '5.0'

  s.script_phase = {
    :name => 'Build Rust library',
    # First argument: path to the crate, relative to this podspec's folder.
    # Second argument: name of the Rust library (crate `dj_api`).
    :script => 'sh "$PODS_TARGET_SRCROOT/../cargokit/build_pod.sh" ../../../native/dj_api dj_api',
    :execution_position => :before_compile,
    :input_files => ['${BUILT_PRODUCTS_DIR}/cargokit_phony'],
    # Let Xcode know that the static library referenced in -force_load below
    # is created by this build step.
    :output_files => ["${BUILT_PRODUCTS_DIR}/libdj_api.a"],
  }
  s.pod_target_xcconfig = {
    'DEFINES_MODULE' => 'YES',
    'OTHER_LDFLAGS' => '-force_load ${BUILT_PRODUCTS_DIR}/libdj_api.a',
  }
end
