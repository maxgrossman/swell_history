#!/bin/bash

if [ -f bouy_history_files.html ]; then 
    wget -O bouy_history_files.html https://www.ndbc.noaa.gov/data/historical/stdmet
fi

# go get timezone shp for database if we do not have it.
if [ ! -f timezones.sqlite ]; then
    wget https://github.com/evansiroky/timezone-boundary-builder/releases/download/2021c/timezones.shapefile.zip 
    unzip timezones.shapefile.zip    
else
    rm -f timezones.sqlite
fi
# load the shapefile into a spatialite db and add the needed index
spatialite -silent timezones.sqlite ".loadshp combined-shapefile timezones CP1252 23032;"
spatialite -silent timezones.sqlite "SELECT CreateSpatialIndex('timezones','geometry');"

# get rid of the bouys db from a previous run then create the bouys table
rm -f bouys.sqlite 

spatialite -silent bouys.sqlite "CREATE TABLE bouys(id text, tzid text);  SELECT AddGeometryColumn('bouys', 'geometry', 4326, 'POINT', 2);"

# run bin that will fetch bouys from ndbc, load them into the bouys table, and associate time zone ids by seeing points in the timezone geomtries
# the cat -> pup -> jq -> awk pipeline gives us a list of the nbdc source files as an easy to parse \n separated list.
cat bouy_history_files.html | pup 'a json{}' | jq ".[].text" | awk '!/_old/' | tr -d '"' | ./target/release/build_bouy_metadata timezones.sqlite