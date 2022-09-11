#!/bin/bash

# bundle the dependencies that do not come bundled.
npx swc node_modules/@mapbox/tilebelt/index.js -o static/assets/tilebelt.min.js
# copy over leaflet
cp node_modules/leaflet/dist/leaflet.js static/assets/leaflet.js
cp node_modules/leaflet/dist/leaflet.css static/assets/leaflet.css
