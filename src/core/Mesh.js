import WebglConst from "../utils/WebglConst";

function Mesh(mDrawType = WebglConst.TRIANGLES) {
  this.drawType = mDrawType;

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
    console.log(mData, bufferData);

    return this;
  };

  this.bufferVertex = function(mData, mUsage = WebglConst.STATIC_DRAW) {
    this.bufferData(mData, "aVertexPosition", 3, mUsage);
    return this;
  };

  this.bufferIndex = function(mData) {
    return this;
  };

  /**
   * add or update an attribute
   *
   * @param {array} mData the data of the attribute, array of array
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
  ) => {};
}

export { Mesh };
