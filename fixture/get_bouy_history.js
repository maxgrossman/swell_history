const {getPromise, utilWriteFile} = require('./utils');
const {parse} = require('node-html-parser');

// take timezones folder, match against what is in historical folder.
// if we don't have anything then ignore. if we do, then read and index data
module.exports = function(bouyHistory) {
  return new Promise(async(resolve) => {
    const bouysIndex = await getPromise('https://www.ndbc.noaa.gov/data/historical/stdmet/')
    if (!bouysIndex) {
      console.log('no bouys to index')
      process.exit(0)
    } else {
      const bouyIds = Object.keys(bouyHistory);
      const root = parse(bouysIndex);
      const tableRecoreds = root.querySelectorAll('tr')
      tableRecoreds.forEach(tr => {
        if (tr.childNodes.length <= 1) return;
        const bouyFile = tr.childNodes[1].querySelector('a').text;
        if (bouyFile && bouyFile.endsWith('txt.gz') && bouyFile.indexOf('_old') === -1) {
          const matchingBouy = bouyIds.find(b => bouyFile.startsWith(b));

          if (!matchingBouy) {
            return;
          }

          bouyHistory[matchingBouy].filenames[bouyFile] = true;
        }
      })

      Object.keys(bouyHistory).forEach(bouy => {
        if (Object.keys(bouyHistory[bouy].filenames).length)
          utilWriteFile(`bouy_metadata/${bouy}`, JSON.stringify(bouyHistory[bouy]))
      })
    }
    resolve()
  })
}
