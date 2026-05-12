#pragma OPENCL EXTENSION cl_khr_int64_base_atomics : enable

__constant ulong KECCAKF_RNDC[24] = {
    0x0000000000000001UL, 0x0000000000008082UL,
    0x800000000000808aUL, 0x8000000080008000UL,
    0x000000000000808bUL, 0x0000000080000001UL,
    0x8000000080008081UL, 0x8000000000008009UL,
    0x000000000000008aUL, 0x0000000000000088UL,
    0x0000000080008009UL, 0x000000008000000aUL,
    0x000000008000808bUL, 0x800000000000008bUL,
    0x8000000000008089UL, 0x8000000000008003UL,
    0x8000000000008002UL, 0x8000000000000080UL,
    0x000000000000800aUL, 0x800000008000000aUL,
    0x8000000080008081UL, 0x8000000000008080UL,
    0x0000000080000001UL, 0x8000000080008008UL
};

__constant uint KECCAKF_ROTC[24] = {
     1,  3,  6, 10, 15, 21, 28, 36,
    45, 55,  2, 14, 27, 41, 56,  8,
    25, 43, 62, 18, 39, 61, 20, 44
};

__constant uint KECCAKF_PILN[24] = {
    10,  7, 11, 17, 18, 3, 5, 16,
     8, 21, 24, 4, 15, 23, 19, 13,
    12,  2, 20, 14, 22, 9, 6, 1
};

inline ulong rol64(ulong x, uint s) {
    return (x << s) | (x >> (64 - s));
}

inline void keccakf(ulong st[25]) {
    ulong bc[5];
    ulong t;

    for (int round = 0; round < 24; round++) {
        for (int i = 0; i < 5; i++) {
            bc[i] = st[i] ^ st[i + 5] ^ st[i + 10] ^ st[i + 15] ^ st[i + 20];
        }

        for (int i = 0; i < 5; i++) {
            t = bc[(i + 4) % 5] ^ rol64(bc[(i + 1) % 5], 1);
            for (int j = 0; j < 25; j += 5) {
                st[j + i] ^= t;
            }
        }

        t = st[1];
        for (int i = 0; i < 24; i++) {
            int j = KECCAKF_PILN[i];
            bc[0] = st[j];
            st[j] = rol64(t, KECCAKF_ROTC[i]);
            t = bc[0];
        }

        for (int j = 0; j < 25; j += 5) {
            for (int i = 0; i < 5; i++) {
                bc[i] = st[j + i];
            }
            for (int i = 0; i < 5; i++) {
                st[j + i] ^= (~bc[(i + 1) % 5]) & bc[(i + 2) % 5];
            }
        }

        st[0] ^= KECCAKF_RNDC[round];
    }
}

inline int digest_less_than_target(const ulong st[25], __global const uchar* target_be) {
    uchar d[32];
    for (int lane = 0; lane < 4; lane++) {
        ulong v = st[lane];
        for (int b = 0; b < 8; b++) {
            d[lane * 8 + b] = (uchar)((v >> (8 * b)) & 0xffUL);
        }
    }

    for (int i = 0; i < 32; i++) {
        uchar a = d[i];
        uchar b = target_be[i];
        if (a < b) return 1;
        if (a > b) return 0;
    }
    return 0;
}

__kernel void mine_keccak(
    __global const uchar* challenge,
    __global const uchar* target_be,
    ulong start_nonce,
    __global volatile int* success_flag,
    __global ulong* found_nonce
) {
    if (*success_flag != 0) return;

    ulong nonce = start_nonce + (ulong)get_global_id(0);

    uchar m[136];
    for (int i = 0; i < 136; i++) m[i] = 0;

    for (int i = 0; i < 32; i++) m[i] = challenge[i];

    m[32] = (uchar)((nonce >> 56) & 0xffUL);
    m[33] = (uchar)((nonce >> 48) & 0xffUL);
    m[34] = (uchar)((nonce >> 40) & 0xffUL);
    m[35] = (uchar)((nonce >> 32) & 0xffUL);
    m[36] = (uchar)((nonce >> 24) & 0xffUL);
    m[37] = (uchar)((nonce >> 16) & 0xffUL);
    m[38] = (uchar)((nonce >> 8) & 0xffUL);
    m[39] = (uchar)(nonce & 0xffUL);

    m[40] = 0x01;
    m[135] = 0x80;

    ulong st[25];
    for (int i = 0; i < 25; i++) st[i] = 0UL;

    for (int lane = 0; lane < 17; lane++) {
        ulong v = 0UL;
        for (int b = 0; b < 8; b++) {
            v |= ((ulong)m[lane * 8 + b]) << (8 * b);
        }
        st[lane] ^= v;
    }

    keccakf(st);

    if (digest_less_than_target(st, target_be)) {
        if (atomic_cmpxchg(success_flag, 0, 1) == 0) {
            found_nonce[0] = nonce;
        }
    }
}
