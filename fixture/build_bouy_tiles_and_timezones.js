const {get} = require('https');
const {chain} = require('stream-chain');
const {parser} = require('stream-json');
const {pick} = require('stream-json/filters/Pick');
const {streamArray} = require('stream-json/streamers/StreamArray');
const {pointToTile} = require('@mapbox/tilebelt');
const {find} = require('geo-tz');
const {utilWriteFile} = require('./utils');

module.exports = function() {
  const zooms = [...Array(5).keys()].map(i => 16 + i),
        tiles = {},
        bouyHistory = {}

  return new Promise(resolve => {
    get('https://www.ndbc.noaa.gov/ndbcmapstations.json', (res) => {
      const pipeline = chain([
        res,
        parser(),
        pick({filter: 'station'}),
        streamArray()
      ])

      pipeline.on('data', (bouy) => {
        if (!bouyHistory[bouy.value.id]) {
          bouyHistory[bouy.value.id] = {
            timezone: find(bouy.value.lat, bouy.value.lon).join(','),
            filenames: {}
          }
        }
        zooms.forEach(zoom => {
          const tileKey = pointToTile(bouy.value.lon, bouy.value.lat, zoom).reverse().join('/')
          if (!tiles.hasOwnProperty(tileKey)) {
            tiles[tileKey] = {
              "type": "FeatureCollection",
              "features": []
            }
          }
          tiles[tileKey].features.push({
            "type": "Feature",
            "properties": {
              id: bouy.value.id,
              name: bouy.value.name,
            },
            "geometry": {
              "type": "Point",
              "coordinates": [bouy.value.lon, bouy.value.lat]
            }
          });
        })
      })

      pipeline.on('end', () => {
        Object.keys(tiles).forEach(tileKey =>
          utilWriteFile(`tiles/${tileKey}.geojson`, JSON.stringify(tiles[tileKey])))

        resolve(bouyHistory);
      })
    })
  })
}
