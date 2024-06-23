# Changelog

All notable changes to this project will be documented in this file. See [standard-version](https://github.com/conventional-changelog/standard-version) for commit guidelines.

## [1.7.1](https://github.com/Akylas/mbview-rs/compare/v1.7.0...v1.7.1) (2024-06-23)

## [1.7.0](https://github.com/Akylas/mbview-rs/compare/v1.6.0...v1.7.0) (2023-05-28)


### Bug Fixes

* default zoom levels ([f9220d4](https://github.com/Akylas/mbview-rs/commit/f9220d4cf1154989a43bd0c30d48b471c6b4fb3e))
* fix for copy tile to geojson ([ca411f5](https://github.com/Akylas/mbview-rs/commit/ca411f5bfa5253b9f73bf8ba0a0a0e0fda551433))
* full reload now working ([4cab82e](https://github.com/Akylas/mbview-rs/commit/4cab82e7252bb7534541d7c6aca8672491435ab4))
* handle more pbf formats ([c27256c](https://github.com/Akylas/mbview-rs/commit/c27256c1f7be20dc267256192d2dcbf3b473d642))
* handle wrong minzoom/maxzoom in metadata ([dcbd568](https://github.com/Akylas/mbview-rs/commit/dcbd56844291de2bd8c581465bb7cea17018de14))
* prevent crash if mbtiles file does not exist ([2cf5e83](https://github.com/Akylas/mbview-rs/commit/2cf5e83024e8d6dac035cd098947220f38b9186e))
* reload data fix ([bbbc9f2](https://github.com/Akylas/mbview-rs/commit/bbbc9f26e6e1d399b0e2e0527b36608923f70bba))
* reload improvement ([3ebbb77](https://github.com/Akylas/mbview-rs/commit/3ebbb7748666cfce1903378a802d5854a1bd08ba))
* reload split position saved ([6ffb3cc](https://github.com/Akylas/mbview-rs/commit/6ffb3cc682d4665894f5dbfef4095ad1e1549673))
* removed menus ([3634a6f](https://github.com/Akylas/mbview-rs/commit/3634a6fc62e4310c0d7d3ff3d4c653adf37dd8ec))
* sort selected items ([8962ccd](https://github.com/Akylas/mbview-rs/commit/8962ccde4ab271ade528f7db6d33af366ed0a035))
* updated tauri ([fd21533](https://github.com/Akylas/mbview-rs/commit/fd215336d831678916bf74ed759e299039333550))


### Features

* hillshade layer support ([7832822](https://github.com/Akylas/mbview-rs/commit/783282270a82d6f48fa27dd790ce6ab8694a02a7))
* last opened folder ([bc7e733](https://github.com/Akylas/mbview-rs/commit/bc7e7331fa68a55839e0a31eb15af9456f8a68cf))



## [1.6.0](https://github.com/Akylas/mbview-rs/compare/v1.5.2...v1.6.0) (2022-05-26)


### Features

* added map context menu ([4311381](https://github.com/Akylas/mbview-rs/commit/4311381f11839a56fc100229874b4973ce2653d8))



## [1.5.2](https://github.com/Akylas/mbview-rs/compare/v1.5.1...v1.5.2) (2022-05-25)


### Bug Fixes

* correctly hide selected layers with toggles ([614c5b4](https://github.com/Akylas/mbview-rs/commit/614c5b44b75dba8c46c7b042ad5a661831c09bc0))
* missing translations ([f2aa541](https://github.com/Akylas/mbview-rs/commit/f2aa54169ec8a7d54f10248ed23b8809d1add24e))
* prevent duplicates in attribute popup ([16a3fd7](https://github.com/Akylas/mbview-rs/commit/16a3fd730cdd9c7b7ffbc2e889b4c42127e8c3bf))
* show geometry type as $type ([0f46557](https://github.com/Akylas/mbview-rs/commit/0f465579947822fbd5040a876814a9e3613e3b40))
* theme style for popup and compare splitter ([f0b13ea](https://github.com/Akylas/mbview-rs/commit/f0b13ea80c60975a59eb895a6d694162dc139a77))
* upgraded deps ([734038d](https://github.com/Akylas/mbview-rs/commit/734038df860d7549c0bee00584691da6591cf640))



## [1.5.1](https://github.com/Akylas/mbview-rs/compare/v1.5.0...v1.5.1) (2022-03-24)


### Bug Fixes

* multiple bugs corrections and improvements ([1af09e5](https://github.com/Akylas/mbview-rs/commit/1af09e5e07517c042ad1f9c01d79b03d3543228e))



## [1.5.0](https://github.com/Akylas/mbview-rs/compare/v1.4.0...v1.5.0) (2022-03-22)


### Features

* new bottom panel to show selected features ([8d9e5ce](https://github.com/Akylas/mbview-rs/commit/8d9e5ce5d6039132aad2b8752299a63a01b73509))



## [1.4.0](https://github.com/Akylas/mbview-rs/compare/v1.3.1...v1.4.0) (2022-03-21)


### Features

* new UI using carbon design with theme support ([10e6021](https://github.com/Akylas/mbview-rs/commit/10e60217a43aac78532f9574d890365fcc506a58))
* allow to add/remove multiple mbtiles per panel
* popup working in both panels

## [1.3.1](https://github.com/Akylas/mbview-rs/compare/v1.3.0...v1.3.1) (2022-03-19)


### Bug Fixes

* bring back tiles cache ([5044028](https://github.com/Akylas/mbview-rs/commit/5044028dee01384974d4e07b2ab68ba5437f4adf))
* fix white screen on production build ([55945e0](https://github.com/Akylas/mbview-rs/commit/55945e020270a5e03ccdfee3c42af397f36997cb))



## [1.3.0](https://github.com/Akylas/mbview-rs/compare/v1.2.0...v1.3.0) (2022-03-18)


### Features

* basic split map view support (select secondary with fab button) ([dd7b5d0](https://github.com/Akylas/mbview-rs/commit/dd7b5d0b19859d2cfc6b38450edea3f324dd278b))
* watch open mbtiles for changes. Will reload the map when changed! ([d0cf700](https://github.com/Akylas/mbview-rs/commit/d0cf70043c94cc9a160daebac7fc5e19e765f971))

## [1.2.0](https://github.com/Akylas/mbview-rs/compare/v1.1.0...v1.2.0) (2022-03-17)


### Features

* localised app (en/fr) ([5fce561](https://github.com/Akylas/mbview-rs/commit/5fce5611f98a9d671c8b143f652b3c8d9033de59))

## [1.1.0](https://github.com/farfromrefug/mbview-rs/compare/v1.0.0...v1.1.0) (2022-03-16)


### Features

* save last opened mbtiles ([a7062e6](https://github.com/farfromrefug/mbview-rs/commit/a7062e6d1d52fc9f09fef3ef6f94e1b3384a4a4e))


### Bug Fixes

* app update ([5499bb5](https://github.com/farfromrefug/mbview-rs/commit/5499bb519276269650bc1be7d42f7ea392aed065))
* map working with popup ([dc70436](https://github.com/farfromrefug/mbview-rs/commit/dc7043664485636e58e56ba97114295866a2e873))
* working custom protocol ([9e7c051](https://github.com/farfromrefug/mbview-rs/commit/9e7c0515f16c7d245ab29e0ad81ced361433e72f))
