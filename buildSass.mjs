import * as sass from 'sass';
import fs from 'fs';

const outputPath = './src/carbon.css';
sass
  .compileAsync('./src/carbon.scss', {
    loadPaths: ['./node_modules'],
  })
  .then((result) => {
    fs.writeFile(outputPath, result.css, function (err) {
      if (err) {
        console.error(err);
        //file written on disk
      }
    });
  })
  .catch((err) => console.error(err));
