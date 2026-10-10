#!/usr/bin/env python3
"""Run rio-rgbify with Rasterio 1.5's stricter transform_bounds validation."""

from rasterio.warp import transform_bounds as rasterio_transform_bounds
import rio_rgbify.mbtiler as mbtiler


def compatible_transform_bounds(*args, **kwargs):
    if kwargs.get("densify_pts") == 0:
        kwargs["densify_pts"] = 2
    return rasterio_transform_bounds(*args, **kwargs)


mbtiler.transform_bounds = compatible_transform_bounds

from rio_rgbify.scripts.cli import rgbify  # noqa: E402


if __name__ == "__main__":
    rgbify()
