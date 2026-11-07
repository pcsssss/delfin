config = configure_file(
  input: 'src/meson_config.rs.in',
  output: 'meson_config.rs',
  configuration: {
    'APP_ID': app_id,
    'BUILDDIR': meson.project_build_root(),
    'PKGDATADIR': pkgdatadir,
    'VERSION': version,
  },
)

run_command(
  'cp',
  meson.project_build_root() / 'delfin' / 'meson_config.rs',
  meson.project_source_root() / 'delfin' / 'src' / 'meson_config.rs',
  check: true,
)

env = environment()
env.set('MESON_BUILD_ROOT', meson.project_build_root())
env.set('FLATPAK', get_option('flatpak') ? 'true' : 'false')

cargo = find_program('cargo', required: true)
clang = find_program('clang', required: false)
mold = find_program('mold', required: false)

cargo_options = ['--target-dir', meson.project_build_root() / 'delfin']
rust_target = 'debug'

if get_option('profile') == 'release'
  cargo_options += ['--release']
  rust_target = 'release'
endif

if clang.found() and mold.found()
  env.append('RUSTFLAGS', '-Clinker=' + clang.full_path(), separator: ' ')
  env.append('RUSTFLAGS', '-Clink-arg=-fuse-ld=' + mold.full_path(), separator: ' ')
endif

cargo_build = custom_target(
  'cargo-build',
  depends: subproject('sys').get_variable('lib'),
  build_always_stale: true,
  output: 'cargo-build',
  console: true,
  env: env,
  command: [
    cargo,
    'build',
    cargo_options,
  ],
)

custom_target(
  'build',
  build_by_default: true,
  build_always_stale: true,
  depends: [cargo_build],
  output: meson.project_name(),
  console: true,
  env: env,
  command: [
    'cp',
    'delfin' / rust_target / meson.project_name(),
    '@OUTPUT@',
  ],
  install: true,
  install_dir: bindir,
)
