import sass from 'node-sass';
import fs from 'fs';

const outputPath = './src/carbon.css';
sass.render(
  {
    file: './src/carbon.scss',
    includePaths: ['./node_modules'],
    outFile: outputPath,
  },
  function (err, result) {
    if (!err) {
      // No errors during the compilation, write this result on the disk
      fs.writeFile(outputPath, result.css, function (err) {
        if (err) {
          console.error(err);
          //file written on disk
        }
      });
    } else {
      console.error(err);
    }
  }
);
