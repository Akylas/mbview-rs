import {extend} from 'maplibre-gl/src/util/util';

import type { Map } from 'maplibre-gl';
import type {ControlPosition, IControl} from 'maplibre-gl';

export type Unit = 'imperial' | 'metric' | 'nautical';

type ScaleOptions = {
    maxWidth?: number;
    unit?: Unit;
};

const defaultOptions: ScaleOptions = {
    maxWidth: 100,
    unit: 'metric'
};

/**
 * A `ScaleControl` control displays the ratio of a distance on the map to the corresponding distance on the ground.
 *
 * @implements {IControl}
 * @param {Object} [options]
 * @param {number} [options.maxWidth='100'] The maximum length of the scale control in pixels.
 * @param {string} [options.unit='metric'] Unit of the distance (`'imperial'`, `'metric'` or `'nautical'`).
 * @example
 * var scale = new maplibregl.ScaleControl({
 *     maxWidth: 80,
 *     unit: 'imperial'
 * });
 * map.addControl(scale);
 *
 * scale.setUnit('metric');
 */
export class ScaleControl implements IControl {
    _map: Map;
    _container: HTMLElement;
    options: ScaleOptions;

    constructor(options: ScaleOptions) {
        this.options = extend({}, defaultOptions, options);

    }

    getDefaultPosition(): ControlPosition {
        return 'bottom-left';
    }

    _onMove = () => {
        updateScale(this._map, this._container, this.options);
    }

    onAdd(map: Map) {
        this._map = map;
        // this._container = DOM.create('div', 'maplibregl-ctrl maplibregl-ctrl-scale mapboxgl-ctrl mapboxgl-ctrl-scale', map.getContainer());
        this._container = document.createElement('div');
        this._container.classList.add('maplibregl-ctrl','maplibregl-ctrl-scale','mapboxgl-ctrl','mapboxgl-ctrl-scale');
        this._map.on('move', this._onMove);
        this._onMove();

        return this._container;
    }

    onRemove() {
        this._container.parentNode.removeChild(this._container);
        this._map.off('move', this._onMove);
        this._map = undefined;
    }

    /**
     * Set the scale's unit of the distance
     *
     * @param unit Unit of the distance (`'imperial'`, `'metric'` or `'nautical'`).
     */
    setUnit = (unit: Unit)=> {
        this.options.unit = unit;
        updateScale(this._map, this._container, this.options);
    }
}

export default ScaleControl;

function updateScale(map, container, options) {
    // A horizontal scale is imagined to be present at center of the map
    // container with maximum length (Default) as 100px.
    // Using spherical law of cosines approximation, the real distance is
    // found between the two coordinates.
    const maxWidth = options && options.maxWidth || 100;

    const y = map._container.clientHeight / 2;
    const left = map.unproject([0, y]);
    const right = map.unproject([maxWidth, y]);
    const maxMeters = left.distanceTo(right);
    // The real distance corresponding to 100px scale length is rounded off to
    // near pretty number and the scale length for the same is found out.
    // Default unit of the scale is based on User's locale.
    if (options && options.unit === 'imperial') {
        const maxFeet = 3.2808 * maxMeters;
        if (maxFeet > 5280) {
            const maxMiles = maxFeet / 5280;
            setScale(map, container, maxWidth, maxMiles, map._getUIString('ScaleControl.Miles'));
        } else {
            setScale(map, container, maxWidth, maxFeet, map._getUIString('ScaleControl.Feet'));
        }
    } else if (options && options.unit === 'nautical') {
        const maxNauticals = maxMeters / 1852;
        setScale(map, container, maxWidth, maxNauticals, map._getUIString('ScaleControl.NauticalMiles'));
    } else if (maxMeters >= 1000) {
        setScale(map, container, maxWidth, maxMeters / 1000, map._getUIString('ScaleControl.Kilometers'));
    } else {
        setScale(map, container, maxWidth, maxMeters, map._getUIString('ScaleControl.Meters'));
    }
}

function setScale(map: Map, container: HTMLDivElement, maxWidth:number, maxDistance:number, unit:Unit) {
    const distance = getRoundNum(maxDistance);
    const ratio = distance / maxDistance;
    container.style.width = `${maxWidth * ratio}px`;
    container.innerHTML = `<b>${map.getZoom().toFixed(1)}</b><br />${distance}&nbsp;${unit}`;
}

function getDecimalRoundNum(d) {
    const multiplier = Math.pow(10, Math.ceil(-Math.log(d) / Math.LN10));
    return Math.round(d * multiplier) / multiplier;
}

function getRoundNum(num) {
    const pow10 = Math.pow(10, (`${Math.floor(num)}`).length - 1);
    let d = num / pow10;

    d = d >= 10 ? 10 :
        d >= 5 ? 5 :
            d >= 3 ? 3 :
                d >= 2 ? 2 :
                    d >= 1 ? 1 : getDecimalRoundNum(d);

    return pow10 * d;
}