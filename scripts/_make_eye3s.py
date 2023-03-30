import os

import numpy as np
from safetensors.numpy import save_file

if __name__ == "__main__":
    eye3 = np.eye(3).astype(np.float32)

    save_file(
        {"pixel_array": eye3},
        "testfiles/eye3.safetensors",
        {"mymeta": "hello from metadata"},
    )

    os.system("rm testfiles/eye3.safetensors_cat3x")
    os.system("touch testfiles/eye3.safetensors_cat3x")
    os.system("cat testfiles/eye3.safetensors >> testfiles/eye3.safetensors_cat3x")
    os.system("cat testfiles/eye3.safetensors >> testfiles/eye3.safetensors_cat3x")
    os.system("cat testfiles/eye3.safetensors >> testfiles/eye3.safetensors_cat3x")
