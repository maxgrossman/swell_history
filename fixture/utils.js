const {get} = require('https');
const {dirname} = require('path');
const {writeFileSync, existsSync, mkdirSync, openSync, closeSync} = require('fs');

exports.getPromise = function(url) {
  return new Promise(resolve => {
      get(url, res => {
        if (res.statusCode !== 200) {
          resolve(null)
        } else {
          let data = '';
          res.on('data', chunk => data += chunk)
          res.on('end', () => resolve(data))
        }
      }).on('error', (e) => {
        console.error(e)
        resolve(null)
      })
  })
}

exports.getResPromise = function(url) {
  return new Promise((resolve, reject) => {
    get(url, res => {
      if (res.statusCode !== 200)
        reject()
      else
        resolve(res)
    })
  })
}

exports.utilWriteFile = function(file, data) {
  let directory = dirname(file);

  if (!existsSync(directory)) {
    mkdirSync(directory, { recursive: true })
  }

  writeFileSync(file, data);
}

exports.utilInitFile = function(file) {
  let directory = dirname(file);

  if (!existsSync(directory)) {
    mkdirSync(directory, { recursive: true })
  }

  if (!existsSync(file)) {
    closeSync(openSync(file, 'w'))
  }
}

exports.localChain = async function(chainFunctions) {
  let ret = null;
  for (let chainFunction of chainFunctions) {
    ret = await chainFunction(ret);
  }
  return ret;
}