/**
 * Common utilities
 * @module glMatrix
 */
var ARRAY_TYPE = typeof Float32Array !== 'undefined' ? Float32Array : Array;
if (!Math.hypot) Math.hypot = function () {
  var y = 0,
      i = arguments.length;

  while (i--) {
    y += arguments[i] * arguments[i];
  }

  return Math.sqrt(y);
};

/**
 * 3x3 Matrix
 * @module mat3
 */

/**
 * Creates a new identity mat3
 *
 * @returns {mat3} a new 3x3 matrix
 */

function create() {
  var out = new ARRAY_TYPE(9);

  if (ARRAY_TYPE != Float32Array) {
    out[1] = 0;
    out[2] = 0;
    out[3] = 0;
    out[5] = 0;
    out[6] = 0;
    out[7] = 0;
  }

  out[0] = 1;
  out[4] = 1;
  out[8] = 1;
  return out;
}

/**
 * 4x4 Matrix<br>Format: column-major, when typed out it looks like row-major<br>The matrices are being post multiplied.
 * @module mat4
 */

/**
 * Creates a new identity mat4
 *
 * @returns {mat4} a new 4x4 matrix
 */

function create$1() {
  var out = new ARRAY_TYPE(16);

  if (ARRAY_TYPE != Float32Array) {
    out[1] = 0;
    out[2] = 0;
    out[3] = 0;
    out[4] = 0;
    out[6] = 0;
    out[7] = 0;
    out[8] = 0;
    out[9] = 0;
    out[11] = 0;
    out[12] = 0;
    out[13] = 0;
    out[14] = 0;
  }

  out[0] = 1;
  out[5] = 1;
  out[10] = 1;
  out[15] = 1;
  return out;
}

const checkWebGL2 = () => {
  const canvas = document.createElement("canvas");
  const ctx =
    canvas.getContext("experimental-webgl2") || canvas.getContext("webgl2");
  return !!ctx;
};

const mobileCheck = () => {
  let isMobile = false;
  if (
    /Android|webOS|iPhone|iPad|iPod|BlackBerry|IEMobile|Opera Mini/i.test(
      navigator.userAgent
    )
  ) {
    isMobile = true;
  }

  return isMobile;
};

const isMobile = mobileCheck();

const getWebGLContext = (mCanvas, mParams, mWebGL2 = false) => {
  if (mWebGL2) {
    return (
      mCanvas.getContext("webgl2", mParams) ||
      mCanvas.getContext("experimental-webgl2", mParams)
    );
  } else {
    return (
      mCanvas.getContext("webgl", mParams) ||
      mCanvas.getContext("experimental-webgl", mParams)
    );
  }
};

const checkViewport = (viewport, x, y, w, h) => {
  let hasChanged = false;
  if (x !== viewport[0]) {
    hasChanged = true;
  }
  if (y !== viewport[1]) {
    hasChanged = true;
  }
  if (w !== viewport[2]) {
    hasChanged = true;
  }
  if (h !== viewport[3]) {
    hasChanged = true;
  }
  return hasChanged;
};

/*
object-assign
(c) Sindre Sorhus
@license MIT
*/
/* eslint-disable no-unused-vars */
var getOwnPropertySymbols = Object.getOwnPropertySymbols;
var hasOwnProperty = Object.prototype.hasOwnProperty;
var propIsEnumerable = Object.prototype.propertyIsEnumerable;

function toObject(val) {
	if (val === null || val === undefined) {
		throw new TypeError('Object.assign cannot be called with null or undefined');
	}

	return Object(val);
}

function shouldUseNative() {
	try {
		if (!Object.assign) {
			return false;
		}

		// Detect buggy property enumeration order in older V8 versions.

		// https://bugs.chromium.org/p/v8/issues/detail?id=4118
		var test1 = new String('abc');  // eslint-disable-line no-new-wrappers
		test1[5] = 'de';
		if (Object.getOwnPropertyNames(test1)[0] === '5') {
			return false;
		}

		// https://bugs.chromium.org/p/v8/issues/detail?id=3056
		var test2 = {};
		for (var i = 0; i < 10; i++) {
			test2['_' + String.fromCharCode(i)] = i;
		}
		var order2 = Object.getOwnPropertyNames(test2).map(function (n) {
			return test2[n];
		});
		if (order2.join('') !== '0123456789') {
			return false;
		}

		// https://bugs.chromium.org/p/v8/issues/detail?id=3056
		var test3 = {};
		'abcdefghijklmnopqrst'.split('').forEach(function (letter) {
			test3[letter] = letter;
		});
		if (Object.keys(Object.assign({}, test3)).join('') !==
				'abcdefghijklmnopqrst') {
			return false;
		}

		return true;
	} catch (err) {
		// We don't expect any of the above to throw, but better to be safe.
		return false;
	}
}

var objectAssign = shouldUseNative() ? Object.assign : function (target, source) {
	var from;
	var to = toObject(target);
	var symbols;

	for (var s = 1; s < arguments.length; s++) {
		from = Object(arguments[s]);

		for (var key in from) {
			if (hasOwnProperty.call(from, key)) {
				to[key] = from[key];
			}
		}

		if (getOwnPropertySymbols) {
			symbols = getOwnPropertySymbols(from);
			for (var i = 0; i < symbols.length; i++) {
				if (propIsEnumerable.call(from, symbols[i])) {
					to[symbols[i]] = from[symbols[i]];
				}
			}
		}
	}

	return to;
};

var defaultGLParameters = {
  alpha: true,
  depth: true,
};

let gl;
class GLTool {
  constructor() {
    this._viewport = [0, 0, 0, 0];
    this._enabledVertexAttribute = [];
    this.identityMatrix = create$1();
    this._normalMatrix = create();
    this._inverseModelViewMatrix = create();
    this._modelMatrix = create$1();
    this._matrix = create$1();
    this._matrixStacks = [];
    this._lastMesh = null;
    this._useWebGL2 = false;
    this._hasArrayInstance = false;
    this._extArrayInstance = false;

    this.isMobile = isMobile;
  }

  // initialize GL from canvas
  init(mSource, mParameters = {}) {
    if (mSource === null || mSource === undefined) {
      console.error("Canvas not exist");
      return;
    }

    if (mSource instanceof WebGLRenderingContext) {
      this.initWithGL(mSource);
    } else if (mSource instanceof WebGL2RenderingContext) {
      this.webgl2 = true;
      this.initWithGL(mSource);
    }

    if (this.canvas !== undefined && this.canvas !== null) {
      this.destroy();
    }

    this.canvas = mSource;
    this.webgl2 = !!mParameters.webgl2 && checkWebGL2();
    const params = objectAssign({}, defaultGLParameters, mParameters);
    const ctx = getWebGLContext(this.canvas, params, this.webgl2);

    this.initWithGL(ctx);
    this.setSize(window.innerWidth, window.innerHeight);
  }

  // initialize GL from WebGLContext
  initWithGL(ctx) {
    if (!this.canvas) {
      this.canvas = ctx.canvas;
    }
    gl = this.gl = ctx;
  }

  // set GL size
  setSize(mWidth, mHeight) {
    this._width = mWidth;
    this._height = mHeight;
    this.canvas.width = this._width;
    this.canvas.height = this._height;
    this._aspectRatio = this._width / this._height;

    if (gl) {
      this.viewport(0, 0, this._width, this._height);
    }
  }

  // clear the GL context
  clear(r, g, b, a) {
    gl.clearColor(r, g, b, a);
    gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);
  }

  // set GL Viewport
  viewport(x, y, w, h) {
    if (checkViewport(this._viewport, w, y, w, h)) {
      gl.viewport(x, y, w, h);
      this._viewport = [x, y, w, h];
    }
  }

  // DESTROY
  destroy() {
    if (this.canvas.parentNode) {
      try {
        this.canvas.parentNode.removeChild(this.canvas);
      } catch (e) {
        console.log("Error : ", e);
      }
    }

    this.canvas = null;
  }
}

const GL = new GLTool();

export { GL, checkWebGL2, getWebGLContext };
