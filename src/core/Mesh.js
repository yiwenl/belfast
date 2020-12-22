import WebglConst from "../utils/WebglConst";

function Mesh(mDrawType = WebglConst.TRIANGLES) {
  this.drawType = mDrawType;

  // PRIVATE PROPERTIES
  this.isInstanced = false;

  // PRIVATE PROPERTIES
  const _attributes = [];
  const _bufferChanged = [];

  // index buffer
  let _usage;
  let _indices;
  let _numItems;

  /**
   * add or update an attribute
   *
   * @param {array} mData the data of the attribute, array of array
   * @param {string} mName the name of the attribute
   * @param {number} mItemSize the size of each element
   * @param {GLenum} mUsage the usage of the attribute, static or dynamic
   * @param {GLenum} isInstanced if the attribute is an instanced attrbute
   */
  this.bufferData = function(
    mData,
    mName,
    mItemSize,
    mUsage = WebglConst.STATIC_DRAW,
    isInstanced = false
  ) {
    const bufferData = mData.flat();
    const itemSize = mItemSize === undefined ? mData[0].length : mItemSize;
    return bufferFlattenData(bufferData, mName, itemSize, mUsage, isInstanced);
  };

  /**
   * Add or Update the vertex position attribute
   *
   * @param {array} mData the data of the vertex positions
   * @param {GLenum} mUsage the usage of the attribute, static or dynamic
   */
  this.bufferVertex = function(mData, mUsage = WebglConst.STATIC_DRAW) {
    return this.bufferData(mData, "aVertexPosition", 3, mUsage);
  };

  /**
   * Add or Update the texture coordinate attribute
   *
   * @param {array} mData the data of the texture coordinate
   * @param {GLenum} mUsage the usage of the attribute, static or dynamic
   */
  this.bufferTexCoord = function(mData, mUsage = WebglConst.STATIC_DRAW) {
    return this.bufferData(mData, "aTextureCoord", 2, mUsage);
  };

  /**
   * Add or Update the vertex normal attribute
   *
   * @param {array} mData the data of the normal
   * @param {GLenum} mUsage the usage of the attribute, static or dynamic
   */
  this.bufferNormal = function(mData, mUsage = WebglConst.STATIC_DRAW) {
    return this.bufferData(mData, "aNormal", 3, mUsage);
  };

  /**
   * Add or Update the index buffer
   *
   * @param {array} mData the data of the index buffer
   * @param {GLenum} mUsage the usage of the attribute, static or dynamic
   */
  this.bufferIndex = function(mData, mUsage = WebglConst.STATIC_DRAW) {
    _usage = mUsage;
    _indices = new Uint16Array(mData);
    _numItems = _indices.length;
    return this;
  };

  /**
   * Find an attribute by name
   *
   * @param {string} mName the name of the attribute
   * @returns {object} the attribute object
   */
  this.getAttribute = function(mName) {
    return _attributes.find((a) => a.name === mName);
  };

  /**
   * Destroy all buffers
   *
   */
  this.destroy = function() {};

  /**
   * add or update an attribute
   *
   * @param {array} mData the data of the attribute
   * @param {string} mName the name of the attribute
   * @param {number} mItemSize the size of each element
   * @param {GLenum} mUsage the usage of the attribute, static or dynamic
   * @param {GLenum} isInstanced if the attribute is an instanced attrbute
   */
  const bufferFlattenData = (
    mData,
    mName,
    mItemSize,
    mUsage = WebglConst.STATIC_DRAW,
    isInstanced = false
  ) => {
    const usage = mUsage;
    this.isInstanced = isInstanced || this.isInstanced;

    const dataArray = new Float32Array(mData);
    const attribute = this.getAttribute(mName);
    console.log("attribute", mName, attribute);

    if (attribute) {
      //	attribute existed, replace with new data
      attribute.itemSize = mItemSize;
      attribute.dataArray = dataArray;
      attribute.source = mData;
    } else {
      //	attribute not exist yet, create new attribute object
      _attributes.push({
        name: mName,
        source: mData,
        itemSize: mItemSize,
        usage,
        dataArray,
        isInstanced,
      });
    }

    _bufferChanged.push(mName);
    return this;
  };
}

export { Mesh };
