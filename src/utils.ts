const toString = Object.prototype.toString;
// default parser function
function parse(x) {
  return x;
}
// gets the item to be sorted
function getItem(this: any, x) {
  const isObject = x != null && typeof x === 'object';
  const isProp = isObject && this.prop in x;
  return this.parser(isProp ? x[this.prop] : x);
}

/**
 * Sorts an array of elements.
 *
 * @param  {Array} array: the collection to sort
 * @param  {Object} cfg: the configuration options
 * @property {String}   cfg.prop: property name (if it is an Array of objects)
 * @property {Boolean}  cfg.desc: determines whether the sort is descending
 * @property {Function} cfg.parser: function to parse the items to expected type
 * @return {Array}
 */
export function sortBy(array, cfg) {
  if (!(array instanceof Array && array.length)) return [];
  if (toString.call(cfg) !== '[object Object]') cfg = {};
  if (typeof cfg.parser !== 'function') cfg.parser = parse;
  cfg.desc = cfg.desc ? -1 : 1;
  return array.sort(function (a, b) {
    a = getItem.call(cfg, a);
    b = getItem.call(cfg, b);
    return cfg.desc * (a < b ? -1 : +(a > b));
  });
}
