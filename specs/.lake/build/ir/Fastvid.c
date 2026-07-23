// Lean compiler output
// Module: Fastvid
// Imports: Init
#include <lean/lean.h>
#if defined(__clang__)
#pragma clang diagnostic ignored "-Wunused-parameter"
#pragma clang diagnostic ignored "-Wunused-label"
#elif defined(__GNUC__) && !defined(__CLANG__)
#pragma GCC diagnostic ignored "-Wunused-parameter"
#pragma GCC diagnostic ignored "-Wunused-label"
#pragma GCC diagnostic ignored "-Wunused-but-set-variable"
#endif
#ifdef __cplusplus
extern "C" {
#endif
lean_object* lean_nat_shiftr(lean_object*, lean_object*);
LEAN_EXPORT lean_object* l___private_Fastvid_0__Fastvid_zigzag_match__1_splitter___boxed(lean_object*, lean_object*, lean_object*, lean_object*);
LEAN_EXPORT lean_object* l___private_Fastvid_0__Fastvid_zigzag_match__1_splitter___redArg___closed__0;
lean_object* lean_nat_to_int(lean_object*);
LEAN_EXPORT lean_object* l___private_Fastvid_0__Fastvid_zigzag_match__1_splitter___redArg(lean_object*, lean_object*, lean_object*);
LEAN_EXPORT lean_object* l_Fastvid_unzigzag(lean_object*);
LEAN_EXPORT lean_object* l___private_Fastvid_0__Fastvid_zigzag_match__1_splitter___redArg___boxed(lean_object*, lean_object*, lean_object*);
LEAN_EXPORT lean_object* l_Fastvid_zigzag___boxed(lean_object*);
lean_object* lean_nat_abs(lean_object*);
uint8_t lean_nat_dec_eq(lean_object*, lean_object*);
LEAN_EXPORT lean_object* l_Fastvid_unzigzag___boxed(lean_object*);
lean_object* lean_nat_mod(lean_object*, lean_object*);
uint8_t lean_int_dec_lt(lean_object*, lean_object*);
lean_object* lean_nat_sub(lean_object*, lean_object*);
lean_object* lean_nat_mul(lean_object*, lean_object*);
lean_object* lean_int_neg_succ_of_nat(lean_object*);
static lean_object* l_Fastvid_zigzag___closed__0;
LEAN_EXPORT lean_object* l___private_Fastvid_0__Fastvid_zigzag_match__1_splitter(lean_object*, lean_object*, lean_object*, lean_object*);
LEAN_EXPORT lean_object* l_Fastvid_zigzag(lean_object*);
lean_object* lean_nat_add(lean_object*, lean_object*);
static lean_object* _init_l_Fastvid_zigzag___closed__0() {
_start:
{
lean_object* x_1; lean_object* x_2; 
x_1 = lean_unsigned_to_nat(0u);
x_2 = lean_nat_to_int(x_1);
return x_2;
}
}
LEAN_EXPORT lean_object* l_Fastvid_zigzag(lean_object* x_1) {
_start:
{
lean_object* x_2; uint8_t x_3; 
x_2 = l_Fastvid_zigzag___closed__0;
x_3 = lean_int_dec_lt(x_1, x_2);
if (x_3 == 0)
{
lean_object* x_4; lean_object* x_5; lean_object* x_6; 
x_4 = lean_nat_abs(x_1);
x_5 = lean_unsigned_to_nat(2u);
x_6 = lean_nat_mul(x_5, x_4);
lean_dec(x_4);
return x_6;
}
else
{
lean_object* x_7; lean_object* x_8; lean_object* x_9; lean_object* x_10; lean_object* x_11; lean_object* x_12; 
x_7 = lean_nat_abs(x_1);
x_8 = lean_unsigned_to_nat(1u);
x_9 = lean_nat_sub(x_7, x_8);
lean_dec(x_7);
x_10 = lean_unsigned_to_nat(2u);
x_11 = lean_nat_mul(x_10, x_9);
lean_dec(x_9);
x_12 = lean_nat_add(x_11, x_8);
lean_dec(x_11);
return x_12;
}
}
}
LEAN_EXPORT lean_object* l_Fastvid_zigzag___boxed(lean_object* x_1) {
_start:
{
lean_object* x_2; 
x_2 = l_Fastvid_zigzag(x_1);
lean_dec(x_1);
return x_2;
}
}
LEAN_EXPORT lean_object* l_Fastvid_unzigzag(lean_object* x_1) {
_start:
{
lean_object* x_2; lean_object* x_3; lean_object* x_4; uint8_t x_5; 
x_2 = lean_unsigned_to_nat(2u);
x_3 = lean_nat_mod(x_1, x_2);
x_4 = lean_unsigned_to_nat(0u);
x_5 = lean_nat_dec_eq(x_3, x_4);
lean_dec(x_3);
if (x_5 == 0)
{
lean_object* x_6; lean_object* x_7; lean_object* x_8; 
x_6 = lean_unsigned_to_nat(1u);
x_7 = lean_nat_shiftr(x_1, x_6);
x_8 = lean_int_neg_succ_of_nat(x_7);
return x_8;
}
else
{
lean_object* x_9; lean_object* x_10; lean_object* x_11; 
x_9 = lean_unsigned_to_nat(1u);
x_10 = lean_nat_shiftr(x_1, x_9);
x_11 = lean_nat_to_int(x_10);
return x_11;
}
}
}
LEAN_EXPORT lean_object* l_Fastvid_unzigzag___boxed(lean_object* x_1) {
_start:
{
lean_object* x_2; 
x_2 = l_Fastvid_unzigzag(x_1);
lean_dec(x_1);
return x_2;
}
}
static lean_object* _init_l___private_Fastvid_0__Fastvid_zigzag_match__1_splitter___redArg___closed__0() {
_start:
{
lean_object* x_1; lean_object* x_2; 
x_1 = lean_unsigned_to_nat(0u);
x_2 = lean_nat_to_int(x_1);
return x_2;
}
}
LEAN_EXPORT lean_object* l___private_Fastvid_0__Fastvid_zigzag_match__1_splitter___redArg(lean_object* x_1, lean_object* x_2, lean_object* x_3) {
_start:
{
lean_object* x_4; uint8_t x_5; 
x_4 = l___private_Fastvid_0__Fastvid_zigzag_match__1_splitter___redArg___closed__0;
x_5 = lean_int_dec_lt(x_1, x_4);
if (x_5 == 0)
{
lean_object* x_6; lean_object* x_7; 
lean_dec(x_3);
x_6 = lean_nat_abs(x_1);
x_7 = lean_apply_1(x_2, x_6);
return x_7;
}
else
{
lean_object* x_8; lean_object* x_9; lean_object* x_10; lean_object* x_11; 
lean_dec(x_2);
x_8 = lean_nat_abs(x_1);
x_9 = lean_unsigned_to_nat(1u);
x_10 = lean_nat_sub(x_8, x_9);
lean_dec(x_8);
x_11 = lean_apply_1(x_3, x_10);
return x_11;
}
}
}
LEAN_EXPORT lean_object* l___private_Fastvid_0__Fastvid_zigzag_match__1_splitter(lean_object* x_1, lean_object* x_2, lean_object* x_3, lean_object* x_4) {
_start:
{
lean_object* x_5; 
x_5 = l___private_Fastvid_0__Fastvid_zigzag_match__1_splitter___redArg(x_2, x_3, x_4);
return x_5;
}
}
LEAN_EXPORT lean_object* l___private_Fastvid_0__Fastvid_zigzag_match__1_splitter___redArg___boxed(lean_object* x_1, lean_object* x_2, lean_object* x_3) {
_start:
{
lean_object* x_4; 
x_4 = l___private_Fastvid_0__Fastvid_zigzag_match__1_splitter___redArg(x_1, x_2, x_3);
lean_dec(x_1);
return x_4;
}
}
LEAN_EXPORT lean_object* l___private_Fastvid_0__Fastvid_zigzag_match__1_splitter___boxed(lean_object* x_1, lean_object* x_2, lean_object* x_3, lean_object* x_4) {
_start:
{
lean_object* x_5; 
x_5 = l___private_Fastvid_0__Fastvid_zigzag_match__1_splitter(x_1, x_2, x_3, x_4);
lean_dec(x_2);
return x_5;
}
}
lean_object* initialize_Init(uint8_t builtin, lean_object*);
static bool _G_initialized = false;
LEAN_EXPORT lean_object* initialize_Fastvid(uint8_t builtin, lean_object* w) {
lean_object * res;
if (_G_initialized) return lean_io_result_mk_ok(lean_box(0));
_G_initialized = true;
res = initialize_Init(builtin, lean_io_mk_world());
if (lean_io_result_is_error(res)) return res;
lean_dec_ref(res);
l_Fastvid_zigzag___closed__0 = _init_l_Fastvid_zigzag___closed__0();
lean_mark_persistent(l_Fastvid_zigzag___closed__0);
l___private_Fastvid_0__Fastvid_zigzag_match__1_splitter___redArg___closed__0 = _init_l___private_Fastvid_0__Fastvid_zigzag_match__1_splitter___redArg___closed__0();
lean_mark_persistent(l___private_Fastvid_0__Fastvid_zigzag_match__1_splitter___redArg___closed__0);
return lean_io_result_mk_ok(lean_box(0));
}
#ifdef __cplusplus
}
#endif
