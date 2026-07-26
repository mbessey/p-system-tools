Disassembling code file tests/FEATURES.CODE
Segment 0 (FEATURED):
  (offset within segment; segment starts at file offset 0x200)
  (SEGTABLE slot 1)
  Procedure 1 (lex level 0, param size 4, data size 82, exit at 0d68):
0ab2  b9 f6              UJP  -10
0ab4  b6 01 03           LOD  1,3
0ab7  a6 21 3d 3d 3d 20 41 70 70 6c 65 20 50 61 73 63 61 6c 20 46 65 61 74 75 72 65 20 44 65 6d 6f 20 3d 3d 3d  LSA  "=== Apple Pascal Feature Demo ==="
0ada  d7                 NOP  
0adb  00                 SLDC  0
0adc  cd 00 13           CXP  0,19
0adf  9e 00              CSP  0
0ae1  b6 01 03           LOD  1,3
0ae4  cd 00 16           CXP  0,22
0ae7  9e 00              CSP  0
0ae9  b6 01 03           LOD  1,3
0aec  cd 00 16           CXP  0,22
0aef  9e 00              CSP  0
0af1  b6 01 03           LOD  1,3
0af4  d7                 NOP  
0af5  a6 11 45 6e 74 65 72 20 79 6f 75 72 20 6e 61 6d 65 3a 20  LSA  "Enter your name: "
0b08  00                 SLDC  0
0b09  cd 00 13           CXP  0,19
0b0c  9e 00              CSP  0
0b0e  b6 01 02           LOD  1,2
0b11  a5 03              LAO  3
0b13  50                 SLDC  80
0b14  cd 00 12           CXP  0,18
0b17  9e 00              CSP  0
0b19  b6 01 02           LOD  1,2
0b1c  cd 00 15           CXP  0,21
0b1f  9e 00              CSP  0
0b21  b6 01 03           LOD  1,3
0b24  d7                 NOP  
0b25  a6 07 48 65 6c 6c 6f 2c 20  LSA  "Hello, "
0b2e  00                 SLDC  0
0b2f  cd 00 13           CXP  0,19
0b32  9e 00              CSP  0
0b34  b6 01 03           LOD  1,3
0b37  a5 03              LAO  3
0b39  00                 SLDC  0
0b3a  cd 00 13           CXP  0,19
0b3d  9e 00              CSP  0
0b3f  b6 01 03           LOD  1,3
0b42  21                 SLDC  33
0b43  00                 SLDC  0
0b44  cd 00 11           CXP  0,17
0b47  9e 00              CSP  0
0b49  b6 01 03           LOD  1,3
0b4c  cd 00 16           CXP  0,22
0b4f  9e 00              CSP  0
0b51  b6 01 03           LOD  1,3
0b54  cd 00 16           CXP  0,22
0b57  9e 00              CSP  0
0b59  b6 01 03           LOD  1,3
0b5c  d7                 NOP  
0b5d  a6 1e 2d 2d 20 4c 6f 6f 70 73 20 28 46 4f 52 2f 57 48 49 4c 45 2f 52 45 50 45 41 54 29 20 2d 2d  LSA  "-- Loops (FOR/WHILE/REPEAT) --"
0b7d  00                 SLDC  0
0b7e  cd 00 13           CXP  0,19
0b81  9e 00              CSP  0
0b83  b6 01 03           LOD  1,3
0b86  cd 00 16           CXP  0,22
0b89  9e 00              CSP  0
0b8b  ce 08              CLP  8
0b8d  b6 01 03           LOD  1,3
0b90  cd 00 16           CXP  0,22
0b93  9e 00              CSP  0
0b95  b6 01 03           LOD  1,3
0b98  d7                 NOP  
0b99  a6 10 2d 2d 20 47 4f 54 4f 2f 4c 41 42 45 4c 20 2d 2d  LSA  "-- GOTO/LABEL --"
0bab  00                 SLDC  0
0bac  cd 00 13           CXP  0,19
0baf  9e 00              CSP  0
0bb1  b6 01 03           LOD  1,3
0bb4  cd 00 16           CXP  0,22
0bb7  9e 00              CSP  0
0bb9  ce 06              CLP  6
0bbb  b6 01 03           LOD  1,3
0bbe  cd 00 16           CXP  0,22
0bc1  9e 00              CSP  0
0bc3  b6 01 03           LOD  1,3
0bc6  d7                 NOP  
0bc7  a6 0a 2d 2d 20 43 41 53 45 20 2d 2d  LSA  "-- CASE --"
0bd3  00                 SLDC  0
0bd4  cd 00 13           CXP  0,19
0bd7  9e 00              CSP  0
0bd9  b6 01 03           LOD  1,3
0bdc  cd 00 16           CXP  0,22
0bdf  9e 00              CSP  0
0be1  02                 SLDC  2
0be2  ce 07              CLP  7
0be4  05                 SLDC  5
0be5  ce 07              CLP  7
0be7  b6 01 03           LOD  1,3
0bea  cd 00 16           CXP  0,22
0bed  9e 00              CSP  0
0bef  b6 01 03           LOD  1,3
0bf2  d7                 NOP  
0bf3  a6 27 2d 2d 20 4e 65 73 74 65 64 20 70 72 6f 63 65 64 75 72 65 73 20 28 6c 65 78 20 6c 65 76 65 6c 20 3e 20 30 29 20 2d 2d  LSA  "-- Nested procedures (lex level > 0) --"
0c1c  00                 SLDC  0
0c1d  cd 00 13           CXP  0,19
0c20  9e 00              CSP  0
0c22  b6 01 03           LOD  1,3
0c25  cd 00 16           CXP  0,22
0c28  9e 00              CSP  0
0c2a  ce 04              CLP  4
0c2c  b6 01 03           LOD  1,3
0c2f  cd 00 16           CXP  0,22
0c32  9e 00              CSP  0
0c34  b6 01 03           LOD  1,3
0c37  a6 0f 2d 2d 20 52 65 63 75 72 73 69 6f 6e 20 2d 2d  LSA  "-- Recursion --"
0c48  d7                 NOP  
0c49  00                 SLDC  0
0c4a  cd 00 13           CXP  0,19
0c4d  9e 00              CSP  0
0c4f  b6 01 03           LOD  1,3
0c52  cd 00 16           CXP  0,22
0c55  9e 00              CSP  0
0c57  b6 01 03           LOD  1,3
0c5a  d7                 NOP  
0c5b  a6 11 20 20 46 61 63 74 6f 72 69 61 6c 28 36 29 20 3d 20  LSA  "  Factorial(6) = "
0c6e  00                 SLDC  0
0c6f  cd 00 13           CXP  0,19
0c72  9e 00              CSP  0
0c74  b6 01 03           LOD  1,3
0c77  06                 SLDC  6
0c78  00                 SLDC  0
0c79  00                 SLDC  0
0c7a  ce 02              CLP  2
0c7c  01                 SLDC  1
0c7d  cd 00 0d           CXP  0,13
0c80  9e 00              CSP  0
0c82  b6 01 03           LOD  1,3
0c85  cd 00 16           CXP  0,22
0c88  9e 00              CSP  0
0c8a  b6 01 03           LOD  1,3
0c8d  cd 00 16           CXP  0,22
0c90  9e 00              CSP  0
0c92  b6 01 03           LOD  1,3
0c95  a6 22 2d 2d 20 41 72 69 74 68 6d 65 74 69 63 2f 6f 72 64 69 6e 61 6c 20 62 75 69 6c 74 2d 69 6e 73 20 2d 2d  LSA  "-- Arithmetic/ordinal built-ins --"
0cb9  d7                 NOP  
0cba  00                 SLDC  0
0cbb  cd 00 13           CXP  0,19
0cbe  9e 00              CSP  0
0cc0  b6 01 03           LOD  1,3
0cc3  cd 00 16           CXP  0,22
0cc6  9e 00              CSP  0
0cc8  ce 09              CLP  9
0cca  b6 01 03           LOD  1,3
0ccd  cd 00 16           CXP  0,22
0cd0  9e 00              CSP  0
0cd2  b6 01 03           LOD  1,3
0cd5  a6 1e 2d 2d 20 53 74 72 69 6e 67 73 20 61 6e 64 20 4c 4f 4e 47 20 49 4e 54 45 47 45 52 20 2d 2d  LSA  "-- Strings and LONG INTEGER --"
0cf5  d7                 NOP  
0cf6  00                 SLDC  0
0cf7  cd 00 13           CXP  0,19
0cfa  9e 00              CSP  0
0cfc  b6 01 03           LOD  1,3
0cff  cd 00 16           CXP  0,22
0d02  9e 00              CSP  0
0d04  ce 0b              CLP  11
0d06  b6 01 03           LOD  1,3
0d09  cd 00 16           CXP  0,22
0d0c  9e 00              CSP  0
0d0e  b6 01 03           LOD  1,3
0d11  a6 0a 2d 2d 20 53 65 74 73 20 2d 2d  LSA  "-- Sets --"
0d1d  d7                 NOP  
0d1e  00                 SLDC  0
0d1f  cd 00 13           CXP  0,19
0d22  9e 00              CSP  0
0d24  b6 01 03           LOD  1,3
0d27  cd 00 16           CXP  0,22
0d2a  9e 00              CSP  0
0d2c  ce 0c              CLP  12
0d2e  b6 01 03           LOD  1,3
0d31  cd 00 16           CXP  0,22
0d34  9e 00              CSP  0
0d36  00                 SLDC  0
0d37  00                 SLDC  0
0d38  cd 00 1d           CXP  0,29
0d3b  b6 01 03           LOD  1,3
0d3e  d7                 NOP  
0d3f  a6 15 3d 3d 3d 20 44 65 6d 6f 20 63 6f 6d 70 6c 65 74 65 20 3d 3d 3d  LSA  "=== Demo complete ==="
0d56  00                 SLDC  0
0d57  cd 00 13           CXP  0,19
0d5a  9e 00              CSP  0
0d5c  b6 01 03           LOD  1,3
0d5f  cd 00 16           CXP  0,22
0d62  9e 00              CSP  0
0d64  01                 SLDC  1
0d65  01                 SLDC  1
0d66  9e 04              CSP  4  {EXIT}
0d68  1f                 SLDC  31
  Procedure 2 (lex level 1, param size 6, data size 0, exit at 0015):
0000  da                 SLDL  3
0001  01                 SLDC  1
0002  c8                 LEQI  
0003  a1 05              FJP  5
0005  01                 SLDC  1
0006  cc 01              STL  1
0008  b9 0b              UJP  11
000a  da                 SLDL  3
000b  da                 SLDL  3
000c  01                 SLDC  1
000d  95                 SBI  
000e  00                 SLDC  0
000f  00                 SLDC  0
0010  cf 02              CGP  2
0012  8f                 MPI  
0013  cc 01              STL  1
0015  ad 01              RNP  1
  Procedure 3 (lex level 1, param size 4, data size 0, exit at 003c):
0022  d9                 SLDL  2
0023  a1 0c              FJP  12
0025  d8                 SLDL  1
0026  d7                 NOP  
0027  a6 04 54 52 55 45  LSA  "TRUE"
002d  aa 50              SAS  80
002f  b9 0b              UJP  11
0031  d8                 SLDL  1
0032  d7                 NOP  
0033  a6 05 46 41 4c 53 45  LSA  "FALSE"
003a  aa 50              SAS  80
003c  ad 00              RNP  0
  Procedure 4 (lex level 1, param size 0, data size 2, exit at 00cd):
0092  0a                 SLDC  10
0093  cc 01              STL  1
0095  ce 05              CLP  5
0097  b6 02 03           LOD  2,3
009a  d7                 NOP  
009b  a6 18 20 20 4c 6f 63 61 6c 56 61 6c 20 61 66 74 65 72 20 49 6e 6e 65 72 3a 20  LSA  "  LocalVal after Inner: "
00b5  00                 SLDC  0
00b6  cd 00 13           CXP  0,19
00b9  9e 00              CSP  0
00bb  b6 02 03           LOD  2,3
00be  d8                 SLDL  1
00bf  00                 SLDC  0
00c0  cd 00 0d           CXP  0,13
00c3  9e 00              CSP  0
00c5  b6 02 03           LOD  2,3
00c8  cd 00 16           CXP  0,22
00cb  9e 00              CSP  0
00cd  ad 00              RNP  0
  Procedure 5 (lex level 2, param size 0, data size 0, exit at 0085):
0048  b6 03 03           LOD  3,3
004b  a6 21 20 20 49 6e 73 69 64 65 20 49 6e 6e 65 72 2c 20 63 61 6c 6c 65 64 20 66 72 6f 6d 20 4f 75 74 65 72  LSA  "  Inside Inner, called from Outer"
006e  d7                 NOP  
006f  00                 SLDC  0
0070  cd 00 13           CXP  0,19
0073  9e 00              CSP  0
0075  b6 03 03           LOD  3,3
0078  cd 00 16           CXP  0,22
007b  9e 00              CSP  0
007d  b6 01 01           LOD  1,1
0080  01                 SLDC  1
0081  82                 ADI  
0082  b8 01 01           STR  1,1
0085  ad 00              RNP  0
  Procedure 6 (lex level 1, param size 0, data size 2, exit at 0136):
00da  00                 SLDC  0
00db  cc 01              STL  1
00dd  d8                 SLDL  1
00de  01                 SLDC  1
00df  82                 ADI  
00e0  cc 01              STL  1
00e2  b6 02 03           LOD  2,3
00e5  a6 06 20 20 4b 20 3d 20  LSA  "  K = "
00ed  d7                 NOP  
00ee  00                 SLDC  0
00ef  cd 00 13           CXP  0,19
00f2  9e 00              CSP  0
00f4  b6 02 03           LOD  2,3
00f7  d8                 SLDL  1
00f8  00                 SLDC  0
00f9  cd 00 0d           CXP  0,13
00fc  9e 00              CSP  0
00fe  b6 02 03           LOD  2,3
0101  cd 00 16           CXP  0,22
0104  9e 00              CSP  0
0106  d8                 SLDL  1
0107  03                 SLDC  3
0108  c9                 LESI  
0109  a1 02              FJP  2
010b  b9 f6              UJP  -10
010d  b6 02 03           LOD  2,3
0110  d7                 NOP  
0111  a6 15 20 20 44 6f 6e 65 20 77 69 74 68 20 47 4f 54 4f 20 64 65 6d 6f  LSA  "  Done with GOTO demo"
0128  00                 SLDC  0
0129  cd 00 13           CXP  0,19
012c  9e 00              CSP  0
012e  b6 02 03           LOD  2,3
0131  cd 00 16           CXP  0,22
0134  9e 00              CSP  0
0136  ad 00              RNP  0
  Procedure 7 (lex level 1, param size 2, data size 0, exit at 0262):
0144  d8                 SLDL  1
0145  b9 7a              UJP  122
0147  b6 02 03           LOD  2,3
014a  d7                 NOP  
014b  a6 02 20 20        LSA  "  "
014f  00                 SLDC  0
0150  cd 00 13           CXP  0,19
0153  9e 00              CSP  0
0155  b6 02 03           LOD  2,3
0158  d8                 SLDL  1
0159  01                 SLDC  1
015a  cd 00 0d           CXP  0,13
015d  9e 00              CSP  0
015f  b6 02 03           LOD  2,3
0162  d7                 NOP  
0163  a6 0d 20 69 73 20 61 20 77 65 65 6b 64 61 79  LSA  " is a weekday"
0172  00                 SLDC  0
0173  cd 00 13           CXP  0,19
0176  9e 00              CSP  0
0178  b6 02 03           LOD  2,3
017b  cd 00 16           CXP  0,22
017e  9e 00              CSP  0
0180  b9 54              UJP  84
0182  b6 02 03           LOD  2,3
0185  a6 02 20 20        LSA  "  "
0189  d7                 NOP  
018a  00                 SLDC  0
018b  cd 00 13           CXP  0,19
018e  9e 00              CSP  0
0190  b6 02 03           LOD  2,3
0193  d8                 SLDL  1
0194  01                 SLDC  1
0195  cd 00 0d           CXP  0,13
0198  9e 00              CSP  0
019a  b6 02 03           LOD  2,3
019d  a6 11 20 69 73 20 61 20 77 65 65 6b 65 6e 64 20 64 61 79  LSA  " is a weekend day"
01b0  d7                 NOP  
01b1  00                 SLDC  0
01b2  cd 00 13           CXP  0,19
01b5  9e 00              CSP  0
01b7  b6 02 03           LOD  2,3
01ba  cd 00 16           CXP  0,22
01bd  9e 00              CSP  0
01bf  b9 15              UJP  21
01c1  ac 00 00 06 00 b9 0e 81 00 83 00 85 00 87 00 89 00 50 00 52 00  XJP  0..6 default 14 table [129, 131, 133, 135, 137, 80, 82]
01d6  d8                 SLDL  1
01d7  b9 7c              UJP  124
01d9  b6 02 03           LOD  2,3
01dc  d7                 NOP  
01dd  a6 14 20 20 69 6e 74 65 67 65 72 2d 63 61 73 65 3a 20 7a 65 72 6f  LSA  "  integer-case: zero"
01f3  00                 SLDC  0
01f4  cd 00 13           CXP  0,19
01f7  9e 00              CSP  0
01f9  b6 02 03           LOD  2,3
01fc  cd 00 16           CXP  0,22
01ff  9e 00              CSP  0
0201  b9 5f              UJP  95
0203  b6 02 03           LOD  2,3
0206  d7                 NOP  
0207  a6 13 20 20 69 6e 74 65 67 65 72 2d 63 61 73 65 3a 20 6f 6e 65  LSA  "  integer-case: one"
021c  00                 SLDC  0
021d  cd 00 13           CXP  0,19
0220  9e 00              CSP  0
0222  b6 02 03           LOD  2,3
0225  cd 00 16           CXP  0,22
0228  9e 00              CSP  0
022a  b9 36              UJP  54
022c  b6 02 03           LOD  2,3
022f  a6 13 20 20 69 6e 74 65 67 65 72 2d 63 61 73 65 3a 20 74 77 6f  LSA  "  integer-case: two"
0244  d7                 NOP  
0245  00                 SLDC  0
0246  cd 00 13           CXP  0,19
0249  9e 00              CSP  0
024b  b6 02 03           LOD  2,3
024e  cd 00 16           CXP  0,22
0251  9e 00              CSP  0
0253  b9 0d              UJP  13
0255  ac 00 00 02 00 b9 06 83 00 5b 00 34 00  XJP  0..2 default 6 table [131, 91, 52]
0262  ad 00              RNP  0
  Procedure 8 (lex level 1, param size 0, data size 4, exit at 0379):
026e  b6 02 03           LOD  2,3
0271  a6 0a 20 20 46 4f 52 20 54 4f 3a 20  LSA  "  FOR TO: "
027d  d7                 NOP  
027e  00                 SLDC  0
027f  cd 00 13           CXP  0,19
0282  9e 00              CSP  0
0284  01                 SLDC  1
0285  cc 01              STL  1
0287  05                 SLDC  5
0288  cc 02              STL  2
028a  d8                 SLDL  1
028b  d9                 SLDL  2
028c  c8                 LEQI  
028d  a1 1b              FJP  27
028f  b6 02 03           LOD  2,3
0292  d8                 SLDL  1
0293  01                 SLDC  1
0294  cd 00 0d           CXP  0,13
0297  9e 00              CSP  0
0299  b6 02 03           LOD  2,3
029c  20                 SLDC  32
029d  00                 SLDC  0
029e  cd 00 11           CXP  0,17
02a1  9e 00              CSP  0
02a3  d8                 SLDL  1
02a4  01                 SLDC  1
02a5  82                 ADI  
02a6  cc 01              STL  1
02a8  b9 f6              UJP  -10
02aa  b6 02 03           LOD  2,3
02ad  cd 00 16           CXP  0,22
02b0  9e 00              CSP  0
02b2  b6 02 03           LOD  2,3
02b5  a6 0e 20 20 46 4f 52 20 44 4f 57 4e 54 4f 3a 20  LSA  "  FOR DOWNTO: "
02c5  d7                 NOP  
02c6  00                 SLDC  0
02c7  cd 00 13           CXP  0,19
02ca  9e 00              CSP  0
02cc  05                 SLDC  5
02cd  cc 01              STL  1
02cf  01                 SLDC  1
02d0  cc 02              STL  2
02d2  d8                 SLDL  1
02d3  d9                 SLDL  2
02d4  c4                 GEQI  
02d5  a1 1b              FJP  27
02d7  b6 02 03           LOD  2,3
02da  d8                 SLDL  1
02db  01                 SLDC  1
02dc  cd 00 0d           CXP  0,13
02df  9e 00              CSP  0
02e1  b6 02 03           LOD  2,3
02e4  20                 SLDC  32
02e5  00                 SLDC  0
02e6  cd 00 11           CXP  0,17
02e9  9e 00              CSP  0
02eb  d8                 SLDL  1
02ec  01                 SLDC  1
02ed  95                 SBI  
02ee  cc 01              STL  1
02f0  b9 f4              UJP  -12
02f2  b6 02 03           LOD  2,3
02f5  cd 00 16           CXP  0,22
02f8  9e 00              CSP  0
02fa  b6 02 03           LOD  2,3
02fd  a6 09 20 20 57 48 49 4c 45 3a 20  LSA  "  WHILE: "
0308  d7                 NOP  
0309  00                 SLDC  0
030a  cd 00 13           CXP  0,19
030d  9e 00              CSP  0
030f  01                 SLDC  1
0310  cc 01              STL  1
0312  d8                 SLDL  1
0313  05                 SLDC  5
0314  c8                 LEQI  
0315  a1 1b              FJP  27
0317  b6 02 03           LOD  2,3
031a  d8                 SLDL  1
031b  01                 SLDC  1
031c  cd 00 0d           CXP  0,13
031f  9e 00              CSP  0
0321  b6 02 03           LOD  2,3
0324  20                 SLDC  32
0325  00                 SLDC  0
0326  cd 00 11           CXP  0,17
0329  9e 00              CSP  0
032b  d8                 SLDL  1
032c  01                 SLDC  1
032d  82                 ADI  
032e  cc 01              STL  1
0330  b9 f2              UJP  -14
0332  b6 02 03           LOD  2,3
0335  cd 00 16           CXP  0,22
0338  9e 00              CSP  0
033a  b6 02 03           LOD  2,3
033d  a6 0a 20 20 52 45 50 45 41 54 3a 20  LSA  "  REPEAT: "
0349  d7                 NOP  
034a  00                 SLDC  0
034b  cd 00 13           CXP  0,19
034e  9e 00              CSP  0
0350  01                 SLDC  1
0351  cc 01              STL  1
0353  b6 02 03           LOD  2,3
0356  d8                 SLDL  1
0357  01                 SLDC  1
0358  cd 00 0d           CXP  0,13
035b  9e 00              CSP  0
035d  b6 02 03           LOD  2,3
0360  20                 SLDC  32
0361  00                 SLDC  0
0362  cd 00 11           CXP  0,17
0365  9e 00              CSP  0
0367  d8                 SLDL  1
0368  01                 SLDC  1
0369  82                 ADI  
036a  cc 01              STL  1
036c  d8                 SLDL  1
036d  05                 SLDC  5
036e  c5                 GRTI  
036f  a1 f0              FJP  -16
0371  b6 02 03           LOD  2,3
0374  cd 00 16           CXP  0,22
0377  9e 00              CSP  0
0379  ad 00              RNP  0
  Procedure 9 (lex level 1, param size 0, data size 92, exit at 065f):
038e  07                 SLDC  7
038f  91                 NGI  
0390  cc 01              STL  1
0392  b6 02 03           LOD  2,3
0395  a6 0c 20 20 41 42 53 28 2d 37 29 20 3d 20  LSA  "  ABS(-7) = "
03a3  d7                 NOP  
03a4  00                 SLDC  0
03a5  cd 00 13           CXP  0,19
03a8  9e 00              CSP  0
03aa  b6 02 03           LOD  2,3
03ad  d8                 SLDL  1
03ae  80                 ABI  
03af  01                 SLDC  1
03b0  cd 00 0d           CXP  0,13
03b3  9e 00              CSP  0
03b5  b6 02 03           LOD  2,3
03b8  cd 00 16           CXP  0,22
03bb  9e 00              CSP  0
03bd  b6 02 03           LOD  2,3
03c0  d7                 NOP  
03c1  a6 0b 20 20 53 51 52 28 36 29 20 3d 20  LSA  "  SQR(6) = "
03ce  00                 SLDC  0
03cf  cd 00 13           CXP  0,19
03d2  9e 00              CSP  0
03d4  b6 02 03           LOD  2,3
03d7  06                 SLDC  6
03d8  98                 SQI  
03d9  01                 SLDC  1
03da  cd 00 0d           CXP  0,13
03dd  9e 00              CSP  0
03df  b6 02 03           LOD  2,3
03e2  cd 00 16           CXP  0,22
03e5  9e 00              CSP  0
03e7  c6 02              LLA  2
03e9  b3 02 00 6c 40 cd  LDC  27648,52544
03ef  cc bd 02           STL  15618
03f2  b6 02 03           LOD  2,3
03f5  a6 0f 20 20 52 4f 55 4e 44 28 33 2e 37 29 20 3d 20  LSA  "  ROUND(3.7) = "
0406  d7                 NOP  
0407  00                 SLDC  0
0408  cd 00 13           CXP  0,19
040b  9e 00              CSP  0
040d  b6 02 03           LOD  2,3
0410  c6 02              LLA  2
0412  bc 02              LDM  2
0414  9e 18              CSP  24
0416  01                 SLDC  1
0417  cd 00 0d           CXP  0,13
041a  9e 00              CSP  0
041c  b6 02 03           LOD  2,3
041f  a6 0f 20 20 54 52 55 4e 43 28 33 2e 37 29 20 3d 20  LSA  "  TRUNC(3.7) = "
0430  d7                 NOP  
0431  00                 SLDC  0
0432  cd 00 13           CXP  0,19
0435  9e 00              CSP  0
0437  b6 02 03           LOD  2,3
043a  c6 02              LLA  2
043c  bc 02              LDM  2
043e  9e 17              CSP  23  {RND}
0440  01                 SLDC  1
0441  cd 00 0d           CXP  0,13
0444  9e 00              CSP  0
0446  b6 02 03           LOD  2,3
0449  cd 00 16           CXP  0,22
044c  9e 00              CSP  0
044e  07                 SLDC  7
044f  c6 06              LLA  6
0451  cf 03              CGP  3
0453  b6 02 03           LOD  2,3
0456  d7                 NOP  
0457  a6 0b 20 20 4f 44 44 28 37 29 20 3d 20  LSA  "  ODD(7) = "
0464  00                 SLDC  0
0465  cd 00 13           CXP  0,19
0468  9e 00              CSP  0
046a  b6 02 03           LOD  2,3
046d  c6 06              LLA  6
046f  00                 SLDC  0
0470  cd 00 13           CXP  0,19
0473  9e 00              CSP  0
0475  08                 SLDC  8
0476  c6 06              LLA  6
0478  cf 03              CGP  3
047a  b6 02 03           LOD  2,3
047d  a6 0b 20 20 4f 44 44 28 38 29 20 3d 20  LSA  "  ODD(8) = "
048a  d7                 NOP  
048b  00                 SLDC  0
048c  cd 00 13           CXP  0,19
048f  9e 00              CSP  0
0491  b6 02 03           LOD  2,3
0494  c6 06              LLA  6
0496  00                 SLDC  0
0497  cd 00 13           CXP  0,19
049a  9e 00              CSP  0
049c  b6 02 03           LOD  2,3
049f  cd 00 16           CXP  0,22
04a2  9e 00              CSP  0
04a4  b6 02 03           LOD  2,3
04a7  a6 10 20 20 50 57 52 4f 46 54 45 4e 28 33 29 20 3d 20  LSA  "  PWROFTEN(3) = "
04b9  d7                 NOP  
04ba  00                 SLDC  0
04bb  cd 00 13           CXP  0,19
04be  9e 00              CSP  0
04c0  b6 02 03           LOD  2,3
04c3  03                 SLDC  3
04c4  9e 24              CSP  36
04c6  0a                 SLDC  10
04c7  02                 SLDC  2
04c8  cd 1f 04           CXP  31,4
04cb  9e 00              CSP  0
04cd  b6 02 03           LOD  2,3
04d0  cd 00 16           CXP  0,22
04d3  9e 00              CSP  0
04d5  41                 SLDC  65
04d6  cc 04              STL  4
04d8  b6 02 03           LOD  2,3
04db  a6 0d 20 20 4f 52 44 28 27 41 27 29 20 3d 20  LSA  "  ORD('A') = "
04ea  d7                 NOP  
04eb  00                 SLDC  0
04ec  cd 00 13           CXP  0,19
04ef  9e 00              CSP  0
04f1  b6 02 03           LOD  2,3
04f4  db                 SLDL  4
04f5  01                 SLDC  1
04f6  cd 00 0d           CXP  0,13
04f9  9e 00              CSP  0
04fb  b6 02 03           LOD  2,3
04fe  d7                 NOP  
04ff  a6 0c 20 20 43 48 52 28 36 36 29 20 3d 20  LSA  "  CHR(66) = "
050d  00                 SLDC  0
050e  cd 00 13           CXP  0,19
0511  9e 00              CSP  0
0513  b6 02 03           LOD  2,3
0516  42                 SLDC  66
0517  00                 SLDC  0
0518  cd 00 11           CXP  0,17
051b  9e 00              CSP  0
051d  b6 02 03           LOD  2,3
0520  cd 00 16           CXP  0,22
0523  9e 00              CSP  0
0525  b6 02 03           LOD  2,3
0528  d7                 NOP  
0529  a6 0e 20 20 53 55 43 43 28 27 41 27 29 20 3d 20  LSA  "  SUCC('A') = "
0539  00                 SLDC  0
053a  cd 00 13           CXP  0,19
053d  9e 00              CSP  0
053f  b6 02 03           LOD  2,3
0542  db                 SLDL  4
0543  01                 SLDC  1
0544  82                 ADI  
0545  00                 SLDC  0
0546  cd 00 11           CXP  0,17
0549  9e 00              CSP  0
054b  b6 02 03           LOD  2,3
054e  d7                 NOP  
054f  a6 0e 20 20 50 52 45 44 28 27 42 27 29 20 3d 20  LSA  "  PRED('B') = "
055f  00                 SLDC  0
0560  cd 00 13           CXP  0,19
0563  9e 00              CSP  0
0565  b6 02 03           LOD  2,3
0568  42                 SLDC  66
0569  01                 SLDC  1
056a  95                 SBI  
056b  00                 SLDC  0
056c  cd 00 11           CXP  0,17
056f  9e 00              CSP  0
0571  b6 02 03           LOD  2,3
0574  cd 00 16           CXP  0,22
0577  9e 00              CSP  0
0579  02                 SLDC  2
057a  cc 05              STL  5
057c  b6 02 03           LOD  2,3
057f  a6 0d 20 20 4f 52 44 28 57 65 64 29 20 3d 20  LSA  "  ORD(Wed) = "
058e  d7                 NOP  
058f  00                 SLDC  0
0590  cd 00 13           CXP  0,19
0593  9e 00              CSP  0
0595  b6 02 03           LOD  2,3
0598  dc                 SLDL  5
0599  01                 SLDC  1
059a  cd 00 0d           CXP  0,13
059d  9e 00              CSP  0
059f  b6 02 03           LOD  2,3
05a2  cd 00 16           CXP  0,22
05a5  9e 00              CSP  0
05a7  dc                 SLDL  5
05a8  01                 SLDC  1
05a9  82                 ADI  
05aa  cc 05              STL  5
05ac  b6 02 03           LOD  2,3
05af  a6 13 20 20 4f 52 44 28 53 55 43 43 28 57 65 64 29 29 20 3d 20  LSA  "  ORD(SUCC(Wed)) = "
05c4  d7                 NOP  
05c5  00                 SLDC  0
05c6  cd 00 13           CXP  0,19
05c9  9e 00              CSP  0
05cb  b6 02 03           LOD  2,3
05ce  dc                 SLDL  5
05cf  01                 SLDC  1
05d0  cd 00 0d           CXP  0,13
05d3  9e 00              CSP  0
05d5  b6 02 03           LOD  2,3
05d8  cd 00 16           CXP  0,22
05db  9e 00              CSP  0
05dd  b6 02 03           LOD  2,3
05e0  d7                 NOP  
05e1  a6 0b 20 20 4d 41 58 49 4e 54 20 3d 20  LSA  "  MAXINT = "
05ee  00                 SLDC  0
05ef  cd 00 13           CXP  0,19
05f2  9e 00              CSP  0
05f4  b6 02 03           LOD  2,3
05f7  c7 ff 7f           LDCI  32767
05fa  01                 SLDC  1
05fb  cd 00 0d           CXP  0,13
05fe  9e 00              CSP  0
0600  b6 02 03           LOD  2,3
0603  cd 00 16           CXP  0,22
0606  9e 00              CSP  0
0608  01                 SLDC  1
0609  c6 06              LLA  6
060b  cf 03              CGP  3
060d  b6 02 03           LOD  2,3
0610  d7                 NOP  
0611  a6 19 20 20 54 52 55 45 20 2f 20 46 41 4c 53 45 20 6c 69 74 65 72 61 6c 73 3a 20  LSA  "  TRUE / FALSE literals: "
062c  00                 SLDC  0
062d  cd 00 13           CXP  0,19
0630  9e 00              CSP  0
0632  b6 02 03           LOD  2,3
0635  c6 06              LLA  6
0637  00                 SLDC  0
0638  cd 00 13           CXP  0,19
063b  9e 00              CSP  0
063d  00                 SLDC  0
063e  c6 06              LLA  6
0640  cf 03              CGP  3
0642  b6 02 03           LOD  2,3
0645  20                 SLDC  32
0646  00                 SLDC  0
0647  cd 00 11           CXP  0,17
064a  9e 00              CSP  0
064c  b6 02 03           LOD  2,3
064f  c6 06              LLA  6
0651  00                 SLDC  0
0652  cd 00 13           CXP  0,19
0655  9e 00              CSP  0
0657  b6 02 03           LOD  2,3
065a  cd 00 16           CXP  0,22
065d  9e 00              CSP  0
065f  ad 00              RNP  0
  Procedure 10 (lex level 1, param size 8, data size 82, exit at 06c0):
066c  c6 01              LLA  1
066e  bc 04              LDM  4
0670  04                 SLDC  4
0671  c6 05              LLA  5
0673  50                 SLDC  80
0674  0c                 SLDC  12
0675  cd 1e 04           CXP  30,4
0678  b6 02 03           LOD  2,3
067b  a6 29 20 20 4c 4f 4e 47 20 49 4e 54 45 47 45 52 20 76 69 61 20 6e 61 6d 65 64 2d 74 79 70 65 20 70 61 72 61 6d 65 74 65 72 3a 20  LSA  "  LONG INTEGER via named-type parameter: "
06a6  d7                 NOP  
06a7  00                 SLDC  0
06a8  cd 00 13           CXP  0,19
06ab  9e 00              CSP  0
06ad  b6 02 03           LOD  2,3
06b0  c6 05              LLA  5
06b2  00                 SLDC  0
06b3  cd 00 13           CXP  0,19
06b6  9e 00              CSP  0
06b8  b6 02 03           LOD  2,3
06bb  cd 00 16           CXP  0,22
06be  9e 00              CSP  0
06c0  ad 00              RNP  0
  Procedure 11 (lex level 1, param size 0, data size 350, exit at 099c):
06cc  c6 53              LLA  83
06ce  d7                 NOP  
06cf  a6 0d 48 65 6c 6c 6f 2c 20 57 6f 72 6c 64 21  LSA  "Hello, World!"
06de  aa 50              SAS  80
06e0  b6 02 03           LOD  2,3
06e3  a6 06 20 20 53 20 3d 20  LSA  "  S = "
06eb  d7                 NOP  
06ec  00                 SLDC  0
06ed  cd 00 13           CXP  0,19
06f0  9e 00              CSP  0
06f2  b6 02 03           LOD  2,3
06f5  c6 53              LLA  83
06f7  00                 SLDC  0
06f8  cd 00 13           CXP  0,19
06fb  9e 00              CSP  0
06fd  b6 02 03           LOD  2,3
0700  d7                 NOP  
0701  a6 0b 20 20 4c 45 4e 47 54 48 20 3d 20  LSA  "  LENGTH = "
070e  00                 SLDC  0
070f  cd 00 13           CXP  0,19
0712  9e 00              CSP  0
0714  b6 02 03           LOD  2,3
0717  c6 53              LLA  83
0719  00                 SLDC  0
071a  be                 LDB  
071b  01                 SLDC  1
071c  cd 00 0d           CXP  0,13
071f  9e 00              CSP  0
0721  b6 02 03           LOD  2,3
0724  cd 00 16           CXP  0,22
0727  9e 00              CSP  0
0729  b6 02 03           LOD  2,3
072c  d7                 NOP  
072d  a6 14 20 20 50 4f 53 28 27 57 6f 72 6c 64 27 2c 20 53 29 20 3d 20  LSA  "  POS('World', S) = "
0743  00                 SLDC  0
0744  cd 00 13           CXP  0,19
0747  9e 00              CSP  0
0749  b6 02 03           LOD  2,3
074c  d7                 NOP  
074d  a6 05 57 6f 72 6c 64  LSA  "World"
0754  c6 53              LLA  83
0756  00                 SLDC  0
0757  00                 SLDC  0
0758  cd 00 1b           CXP  0,27
075b  01                 SLDC  1
075c  cd 00 0d           CXP  0,13
075f  9e 00              CSP  0
0761  b6 02 03           LOD  2,3
0764  cd 00 16           CXP  0,22
0767  9e 00              CSP  0
0769  c6 2a              LLA  42
076b  c6 53              LLA  83
076d  c6 80 80           LLA  128
0770  08                 SLDC  8
0771  05                 SLDC  5
0772  cd 00 19           CXP  0,25
0775  c6 80 80           LLA  128
0778  aa 50              SAS  80
077a  b6 02 03           LOD  2,3
077d  a6 10 20 20 43 4f 50 59 28 53 2c 38 2c 35 29 20 3d 20  LSA  "  COPY(S,8,5) = "
078f  d7                 NOP  
0790  00                 SLDC  0
0791  cd 00 13           CXP  0,19
0794  9e 00              CSP  0
0796  b6 02 03           LOD  2,3
0799  c6 2a              LLA  42
079b  00                 SLDC  0
079c  cd 00 13           CXP  0,19
079f  9e 00              CSP  0
07a1  b6 02 03           LOD  2,3
07a4  cd 00 16           CXP  0,22
07a7  9e 00              CSP  0
07a9  c6 01              LLA  1
07ab  00                 SLDC  0
07ac  cc 80 80           STL  128
07af  c6 80 80           LLA  128
07b2  d7                 NOP  
07b3  a6 07 50 72 65 66 69 78 2d  LSA  "Prefix-"
07bc  07                 SLDC  7
07bd  cd 00 17           CXP  0,23
07c0  c6 80 80           LLA  128
07c3  c6 53              LLA  83
07c5  57                 SLDC  87
07c6  cd 00 17           CXP  0,23
07c9  c6 80 80           LLA  128
07cc  d7                 NOP  
07cd  a6 07 2d 53 75 66 66 69 78  LSA  "-Suffix"
07d6  5e                 SLDC  94
07d7  cd 00 17           CXP  0,23
07da  c6 80 80           LLA  128
07dd  aa 50              SAS  80
07df  b6 02 03           LOD  2,3
07e2  d7                 NOP  
07e3  a6 0b 20 20 43 4f 4e 43 41 54 20 3d 20  LSA  "  CONCAT = "
07f0  00                 SLDC  0
07f1  cd 00 13           CXP  0,19
07f4  9e 00              CSP  0
07f6  b6 02 03           LOD  2,3
07f9  c6 01              LLA  1
07fb  00                 SLDC  0
07fc  cd 00 13           CXP  0,19
07ff  9e 00              CSP  0
0801  b6 02 03           LOD  2,3
0804  cd 00 16           CXP  0,22
0807  9e 00              CSP  0
0809  c6 01              LLA  1
080b  01                 SLDC  1
080c  07                 SLDC  7
080d  cd 00 1a           CXP  0,26
0810  b6 02 03           LOD  2,3
0813  a6 11 20 20 61 66 74 65 72 20 44 45 4c 45 54 45 20 3d 20  LSA  "  after DELETE = "
0826  d7                 NOP  
0827  00                 SLDC  0
0828  cd 00 13           CXP  0,19
082b  9e 00              CSP  0
082d  b6 02 03           LOD  2,3
0830  c6 01              LLA  1
0832  00                 SLDC  0
0833  cd 00 13           CXP  0,19
0836  9e 00              CSP  0
0838  b6 02 03           LOD  2,3
083b  cd 00 16           CXP  0,22
083e  9e 00              CSP  0
0840  d7                 NOP  
0841  a6 04 4e 45 57 2d  LSA  "NEW-"
0847  c6 01              LLA  1
0849  50                 SLDC  80
084a  01                 SLDC  1
084b  cd 00 18           CXP  0,24
084e  b6 02 03           LOD  2,3
0851  a6 11 20 20 61 66 74 65 72 20 49 4e 53 45 52 54 20 3d 20  LSA  "  after INSERT = "
0864  d7                 NOP  
0865  00                 SLDC  0
0866  cd 00 13           CXP  0,19
0869  9e 00              CSP  0
086b  b6 02 03           LOD  2,3
086e  c6 01              LLA  1
0870  00                 SLDC  0
0871  cd 00 13           CXP  0,19
0874  9e 00              CSP  0
0876  b6 02 03           LOD  2,3
0879  cd 00 16           CXP  0,22
087c  9e 00              CSP  0
087e  c6 7c              LLA  124
0880  c7 39 30           LDCI  12345
0883  12                 SLDC  18
0884  cd 1e 04           CXP  30,4
0887  c7 10 27           LDCI  10000
088a  12                 SLDC  18
088b  cd 1e 04           CXP  30,4
088e  08                 SLDC  8
088f  cd 1e 04           CXP  30,4
0892  c7 85 1a           LDCI  6789
0895  12                 SLDC  18
0896  cd 1e 04           CXP  30,4
0899  02                 SLDC  2
089a  cd 1e 04           CXP  30,4
089d  c7 e8 03           LDCI  1000
08a0  12                 SLDC  18
08a1  cd 1e 04           CXP  30,4
08a4  08                 SLDC  8
08a5  cd 1e 04           CXP  30,4
08a8  0c                 SLDC  12
08a9  12                 SLDC  18
08aa  cd 1e 04           CXP  30,4
08ad  02                 SLDC  2
08ae  cd 1e 04           CXP  30,4
08b1  04                 SLDC  4
08b2  00                 SLDC  0
08b3  cd 1e 04           CXP  30,4
08b6  bd 04              STM  4
08b8  c6 7c              LLA  124
08ba  bc 04              LDM  4
08bc  04                 SLDC  4
08bd  c6 2a              LLA  42
08bf  50                 SLDC  80
08c0  0c                 SLDC  12
08c1  cd 1e 04           CXP  30,4
08c4  b6 02 03           LOD  2,3
08c7  a6 11 20 20 53 54 52 28 4c 6f 6e 67 49 6e 74 29 20 3d 20  LSA  "  STR(LongInt) = "
08da  d7                 NOP  
08db  00                 SLDC  0
08dc  cd 00 13           CXP  0,19
08df  9e 00              CSP  0
08e1  b6 02 03           LOD  2,3
08e4  c6 2a              LLA  42
08e6  00                 SLDC  0
08e7  cd 00 13           CXP  0,19
08ea  9e 00              CSP  0
08ec  b6 02 03           LOD  2,3
08ef  cd 00 16           CXP  0,22
08f2  9e 00              CSP  0
08f4  c7 94 26           LDCI  9876
08f7  12                 SLDC  18
08f8  cd 1e 04           CXP  30,4
08fb  c7 10 27           LDCI  10000
08fe  12                 SLDC  18
08ff  cd 1e 04           CXP  30,4
0902  08                 SLDC  8
0903  cd 1e 04           CXP  30,4
0906  c7 38 15           LDCI  5432
0909  12                 SLDC  18
090a  cd 1e 04           CXP  30,4
090d  02                 SLDC  2
090e  cd 1e 04           CXP  30,4
0911  c7 10 27           LDCI  10000
0914  12                 SLDC  18
0915  cd 1e 04           CXP  30,4
0918  08                 SLDC  8
0919  cd 1e 04           CXP  30,4
091c  c7 4a 04           LDCI  1098
091f  12                 SLDC  18
0920  cd 1e 04           CXP  30,4
0923  02                 SLDC  2
0924  cd 1e 04           CXP  30,4
0927  04                 SLDC  4
0928  00                 SLDC  0
0929  cd 1e 04           CXP  30,4
092c  cf 0a              CGP  10
092e  c6 53              LLA  83
0930  c6 53              LLA  83
0932  af 04              EQU  4
0934  a1 2b              FJP  43
0936  b6 02 03           LOD  2,3
0939  a6 17 20 20 73 74 72 69 6e 67 20 65 71 75 61 6c 69 74 79 20 77 6f 72 6b 73  LSA  "  string equality works"
0952  d7                 NOP  
0953  00                 SLDC  0
0954  cd 00 13           CXP  0,19
0957  9e 00              CSP  0
0959  b6 02 03           LOD  2,3
095c  cd 00 16           CXP  0,22
095f  9e 00              CSP  0
0961  a6 03 41 42 43     LSA  "ABC"
0966  d7                 NOP  
0967  a6 03 41 42 44     LSA  "ABD"
096c  d7                 NOP  
096d  b5 04              LES  4
096f  a1 2b              FJP  43
0971  b6 02 03           LOD  2,3
0974  d7                 NOP  
0975  a6 17 20 20 73 74 72 69 6e 67 20 6f 72 64 65 72 69 6e 67 20 77 6f 72 6b 73  LSA  "  string ordering works"
098e  00                 SLDC  0
098f  cd 00 13           CXP  0,19
0992  9e 00              CSP  0
0994  b6 02 03           LOD  2,3
0997  cd 00 16           CXP  0,22
099a  9e 00              CSP  0
099c  ad 00              RNP  0
  Procedure 12 (lex level 1, param size 0, data size 12, exit at 0aa5):
09a8  1f                 SLDC  31
09a9  01                 SLDC  1
09aa  a0 01              ADJ  1
09ac  cc 03              STL  3
09ae  60                 SLDC  96
09af  01                 SLDC  1
09b0  a0 01              ADJ  1
09b2  cc 02              STL  2
09b4  da                 SLDL  3
09b5  01                 SLDC  1
09b6  d9                 SLDL  2
09b7  01                 SLDC  1
09b8  8c                 INT  
09b9  a0 01              ADJ  1
09bb  cc 01              STL  1
09bd  00                 SLDC  0
09be  da                 SLDL  3
09bf  01                 SLDC  1
09c0  8b                 INN  
09c1  a1 26              FJP  38
09c3  b6 02 03           LOD  2,3
09c6  d7                 NOP  
09c7  a6 12 20 20 4d 6f 6e 20 69 73 20 61 20 77 65 65 6b 64 61 79  LSA  "  Mon is a weekday"
09db  00                 SLDC  0
09dc  cd 00 13           CXP  0,19
09df  9e 00              CSP  0
09e1  b6 02 03           LOD  2,3
09e4  cd 00 16           CXP  0,22
09e7  9e 00              CSP  0
09e9  05                 SLDC  5
09ea  da                 SLDL  3
09eb  01                 SLDC  1
09ec  8b                 INN  
09ed  93                 LNOT  
09ee  a1 2a              FJP  42
09f0  b6 02 03           LOD  2,3
09f3  a6 16 20 20 53 61 74 20 69 73 20 6e 6f 74 20 61 20 77 65 65 6b 64 61 79  LSA  "  Sat is not a weekday"
0a0b  d7                 NOP  
0a0c  00                 SLDC  0
0a0d  cd 00 13           CXP  0,19
0a10  9e 00              CSP  0
0a12  b6 02 03           LOD  2,3
0a15  cd 00 16           CXP  0,22
0a18  9e 00              CSP  0
0a1a  d8                 SLDL  1
0a1b  01                 SLDC  1
0a1c  00                 SLDC  0
0a1d  af 08              EQU  8
0a1f  a1 39              FJP  57
0a21  b6 02 03           LOD  2,3
0a24  d7                 NOP  
0a25  a6 25 20 20 77 65 65 6b 64 61 79 73 20 61 6e 64 20 77 65 65 6b 65 6e 64 20 64 6f 20 6e 6f 74 20 6f 76 65 72 6c 61 70  LSA  "  weekdays and weekend do not overlap"
0a4c  00                 SLDC  0
0a4d  cd 00 13           CXP  0,19
0a50  9e 00              CSP  0
0a52  b6 02 03           LOD  2,3
0a55  cd 00 16           CXP  0,22
0a58  9e 00              CSP  0
0a5a  1f                 SLDC  31
0a5b  01                 SLDC  1
0a5c  a0 01              ADJ  1
0a5e  cc 06              STL  6
0a60  c7 f8 03           LDCI  1016
0a63  01                 SLDC  1
0a64  a0 01              ADJ  1
0a66  cc 05              STL  5
0a68  dd                 SLDL  6
0a69  01                 SLDC  1
0a6a  dc                 SLDL  5
0a6b  01                 SLDC  1
0a6c  8c                 INT  
0a6d  a0 01              ADJ  1
0a6f  cc 04              STL  4
0a71  db                 SLDL  4
0a72  01                 SLDC  1
0a73  18                 SLDC  24
0a74  01                 SLDC  1
0a75  af 08              EQU  8
0a77  a1 2c              FJP  44
0a79  b6 02 03           LOD  2,3
0a7c  d7                 NOP  
0a7d  a6 18 20 20 73 65 74 20 69 6e 74 65 72 73 65 63 74 69 6f 6e 20 77 6f 72 6b 73  LSA  "  set intersection works"
0a97  00                 SLDC  0
0a98  cd 00 13           CXP  0,19
0a9b  9e 00              CSP  0
0a9d  b6 02 03           LOD  2,3
0aa0  cd 00 16           CXP  0,22
0aa3  9e 00              CSP  0
0aa5  ad 00              RNP  0

