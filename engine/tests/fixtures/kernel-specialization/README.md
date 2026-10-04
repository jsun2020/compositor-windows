# Synthetic kernel output references

Generated before kernel optimization from production engine source at
`e644fe363ae6d2ad966c32d22ddf9581862e6a91`, using the already validated
release library (SHA256 `b613f0e1a1440b8f2e608adeedfa8c50f30ac62b2923d38c044c4906f456bb2f`).

Inputs are defined in `../../support/kernel_probe_inputs.rs`. The paint
reference concatenates each touched flag and its complete gray/RGBA output
for Fill, Linear and Radial paints, partial/absent coverage, rotated/flipped
placement and off-canvas refusal. The spatial reference concatenates complete
Gaussian and Motion Blur results with odd sizes, varying premultiplied alpha
and multiple reduction levels. Both references are entirely synthetic.

- paint.rgba: 66,168 bytes; SHA256 ef92921c7ce2a03f9cd5a0a6cbb71687088291b1186c51972c67e8036b13b876
- spatial.rgba: 71,440 bytes; SHA256 92ba2e0f12ee7f4015e21d9b28ef15a92ab9d0edddac2dd56f01baf7b828ae90
