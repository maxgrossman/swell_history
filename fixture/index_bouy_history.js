const indexBouyYear = require('./index_bouy_year');
const {promises} = require('fs');

module.exports = function (bouy) {
  return new Promise(resolve => {
    promises.readFile(`bouy_metadata/${bouy}`).then(async (bouyMetadata) => {
      const {timezone, filenames} = JSON.parse(bouyMetadata.toString());
      for (const filename in filenames) {
        await indexBouyYear(bouy, timezone, filename)
      }
      resolve()
    })
  })
  .then()
}
