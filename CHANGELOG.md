# Changelog

All notable changes to this project will be documented in this file.

## [0.5.2](https://github.com/paulkre/bevy_image_export/compare/v0.5.1...v0.5.2) (2023-03-16)

### Bug Fixes

- Fixed usage of output image widths that are not a multiple of 256.
- Cleaned up codebase.

## [0.5.1](https://github.com/paulkre/bevy_image_export/compare/v0.5.0...v0.5.1) (2023-03-14)

### Bug Fixes

- Fixed usage of output image resolutions that are bigger than the viewport's resolution.

## [0.5.0](https://github.com/paulkre/bevy_image_export/compare/v0.4.1...v0.5.0) (2023-03-14)

### Features

- Added support for specification of output image resolution.

### Bug Fixes

- Improved ECS structure.
- Cleaned up code of examples.
- Cleaned up code related to Bevy's render node system.

## [0.4.1](https://github.com/paulkre/bevy_image_export/compare/v0.4.0...v0.4.1) (2023-03-07)

### Bug Fixes

- Reduced dependency on Bevy to only necessary parts (`bevy_render`, `bevy_asset`).

## [0.4.0](https://github.com/paulkre/bevy_image_export/compare/v0.3.0...v0.4.0) (2023-03-07)

### Features

- Added support for Bevy 0.10.

### Bug Fixes

- Improved efficiency of concurrency.
- Added shields to readme.
- Improved crate metadata.
- Improved ECS structure.
- Changed color format of recorded frames from BGRA to RGBA.
- Cleaned up codebase.

## [0.3.0](https://github.com/paulkre/bevy_image_export/compare/v0.2.2...v0.3.0) (2022-11-18)

### Features

- Added support for Bevy 0.9.

## [0.2.2](https://github.com/paulkre/bevy_image_export/compare/v0.2.1...v0.2.2) (2022-09-21)

## Bug Fixes

- Fixed the export of images with non-uniform resolutions.

## [0.2.1](https://github.com/paulkre/bevy_image_export/compare/v0.2.0...v0.2.1) (2022-09-15)

### Bug Fixes

- Fixed a bug for cameras that were spawned after initial startup. Previously, the frames exported from those cameras had incorrect frame numbers.
- Added documentation link to crate metadata.

## [0.2.0](https://github.com/paulkre/bevy_image_export/compare/v0.1.0...v0.2.0) (2022-09-10)

### Features

- Added example showing usage of plugin for 2D setups.

### Bug Fixes

- Improved wording of readme.
- Added documentation to public aspects of crate.
- Cleaned up code for converting color information in exported images.
- Added explanation about MP4 conversion to readme.
- Improved ECS structure.
- Fixed crate metadata.

## 0.1.0 (2022-09-07)

This is the initial release of the plugin. It allows the user to add the `ImageExportCamera` component to a camera entity to turn that camera into a *recorder*, that saves every frame as an image file on disk.  

