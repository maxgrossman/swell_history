const {get} = require('https')
const zlib     = require('zlib');
const readline = require('readline');
const {v4} = require('uuid');
const {writeFileSync, existsSync, mkdirSync, statSync, appendFileSync} = require('fs');
const {dirname} = require('path');
const {join} = require('path');
const {DateTime} = require('luxon');
const { getResPromise, utilInitFile } = require('./utils');
const {interleave} = require('./interleave/dist');

function makeTimestamp(year, month, day, hour, minute, timezone) {
  return DateTime.fromObject({
    year: year, month: month,
    day: day, hour: hour, minute: minute
  }, { zone: timezone }).toString();
}


module.exports = function(bouyId, timezone, filename) {
  const swellSignatures = {}, bouysMap = {}
  const indexes = {};

  function readLine(line) {
    const data = line.split(' ').filter(d => d.length);
    const swellDirection = Number(data[indexes.MWD]);
    if (swellDirection > 360 || swellDirection < 0) {
      return
    }
    const timestamp = makeTimestamp(
      year = data[indexes.YY], month = data[indexes.MM],
      day = data[indexes.DD], hour = data[indexes.hh],
      minute = data[indexes.mm], timezone
    )

    const swellPeriod = Math.round(Number(data[indexes.APD]) * 100);
    const waveHeight = Math.round(Number(data[indexes.WVHT]) * 100);
gn
    const zIndex = interleave(waveHeight, swellPeriod, swellDirection)
    // create the unique id for the station data
    if (!swellSignatures.hasOwnProperty(zIndex))
      swellSignatures[zIndex] = []

    swellSignatures[zIndex].push(timestamp);

    bouysMap[timestamp] = {
        timestamp: timestamp,
        wdir: data[indexes.WDIR],
        wspd: data[indexes.WSPD],
        gst: data[indexes.GST], wvht: data[indexes.WVHT],
        dpd: data[indexes.DPD], apd: data[indexes.APD], mwd: data[indexes.MWD],
        pres: data[indexes.PRES], atmp: data[indexes.ATMP],
        wtmp: data[indexes.WTMP], dewp: data[indexes.DEWP],
        vis: data[indexes.VIS], tide: data[indexes.TIDE]
    }
  }

  return new Promise(async (resolve, reject) => {
    try {
      const bouyRes = await getResPromise(`https://www.ndbc.noaa.gov/data/historical/stdmet/${filename}`)
      let year = filename.replace('.txt.gz','').split('h').pop()

      console.log(`Reading file ${filename}`)

      const lineReader = readline.createInterface({ input: bouyRes.pipe(zlib.createGunzip()) });
      let readingData = false, firstLine = true
      lineReader.on('line', (line) => {
        if (firstLine) {
          firstLine = false
          line.split(' ').filter(d => d.length)
            .forEach((key, index) => indexes[key.replace('#','')] = index)

        }
        if (readingData) {
          readLine(line)
        } else if (!readingData && line.startsWith(year)) {
g         readingData = true;
          readLine(line)
        }
      })
      lineReader.on('close', () => {
        Object.keys(bouysMap).forEach(readingTimestamp => {
          const file = `bouys/${bouyId}/readings/${readingTimestamp}`
          const directory = dirname(file)
          if (!existsSync(directory)) {
            mkdirSync(directory, { recursive: true })
          }
          writeFileSync(file, JSON.stringify(bouysMap[readingTimestamp]))
        })
        Object.keys(swellSignatures).forEach(zIndex => {
          const file = `bouys/${bouyId}/index/${zIndex}`
          utilInitFile(file);
          let nextFileData = '';
          if (statSync(file).size > 0) {
            nextFileData += '\n'
          }

          nextFileData += swellSignatures[zIndex].join('\n');
          appendFileSync(file, nextFileData)
        })
        resolve();
      })
    } catch (e) {
      console.log(e);
      process.exit(1);
    }
  })
}gpgn
