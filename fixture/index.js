const {localChain} = require('./utils');

localChain([
  require('./build_bouy_tiles_and_timezones'),
  require('./get_bouy_history')
])
.then(() => {
  console.log('...bouys indexed')
  process.exit(0)
})
