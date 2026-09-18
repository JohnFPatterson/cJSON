//! Behavioral port of `cJSON_Utils.c` with C-ABI exports matching `cJSON_Utils.h`.
//!
//! Intentionally preserves C quirks (e.g. `decode_pointer_inplace` `~1` handling,
//! case-insensitive detach in `detach_path`, merge-patch key/`strcmp` behavior).

use cjson::{
    cJSON, cJSON_bool, cJSON_Array, cJSON_Invalid, cJSON_Number, cJSON_Object, cJSON_String,
    cJSON_AddItemToArray, cJSON_AddItemToObject, cJSON_CreateArray, cJSON_CreateNull,
    cJSON_CreateObject, cJSON_CreateString, cJSON_Delete, cJSON_DeleteItemFromObject,
    cJSON_DeleteItemFromObjectCaseSensitive, cJSON_DetachItemFromObject,
    cJSON_DetachItemFromObjectCaseSensitive, cJSON_Duplicate, cJSON_GetObjectItem,
    cJSON_GetObjectItemCaseSensitive, cJSON_IsArray, cJSON_IsNull, cJSON_IsObject,
    cJSON_IsString, cJSON_free, cJSON_malloc,
};
use libc::{c_char, c_int, size_t, strcmp, strlen, tolower};
use std::ptr;

const ULONG_MAX: u64 = u64::MAX;
const DBL_EPSILON: f64 = f64::EPSILON;

#[inline]
fn c_true() -> cJSON_bool {
    1
}
#[inline]
fn c_false() -> cJSON_bool {
    0
}

unsafe fn utils_strdup(string: *const u8) -> *mut u8 {
    if string.is_null() {
        return ptr::null_mut();
    }
    let length = strlen(string as *const c_char) + 1;
    let copy = cJSON_malloc(length) as *mut u8;
    if copy.is_null() {
        return ptr::null_mut();
    }
    ptr::copy_nonoverlapping(string, copy, length);
    copy
}

/// String comparison which doesn't consider NULL pointers equal.
unsafe fn compare_strings(
    string1: *const u8,
    string2: *const u8,
    case_sensitive: cJSON_bool,
) -> c_int {
    if string1.is_null() || string2.is_null() {
        return 1;
    }
    if string1 == string2 {
        return 0;
    }
    if case_sensitive != 0 {
        return strcmp(string1 as *const c_char, string2 as *const c_char);
    }

    let mut s1 = string1;
    let mut s2 = string2;
    loop {
        let c1 = tolower(*s1 as c_int);
        let c2 = tolower(*s2 as c_int);
        if c1 != c2 {
            return c1 - c2;
        }
        if *s1 == 0 {
            return 0;
        }
        s1 = s1.add(1);
        s2 = s2.add(1);
    }
}

unsafe fn compare_double(a: f64, b: f64) -> cJSON_bool {
    let max_val = if a.abs() > b.abs() { a.abs() } else { b.abs() };
    if (a - b).abs() <= max_val * DBL_EPSILON {
        c_true()
    } else {
        c_false()
    }
}

/// Compare the next path element of two JSON pointers; two NULL pointers are unequal.
unsafe fn compare_pointers(
    mut name: *const u8,
    mut pointer: *const u8,
    case_sensitive: cJSON_bool,
) -> cJSON_bool {
    if name.is_null() || pointer.is_null() {
        return c_false();
    }

    while *name != 0 && *pointer != 0 && *pointer != b'/' {
        if *pointer == b'~' {
            if ((*pointer.add(1) != b'0') || (*name != b'~'))
                && ((*pointer.add(1) != b'1') || (*name != b'/'))
            {
                return c_false();
            } else {
                pointer = pointer.add(1);
            }
        } else if (case_sensitive == 0
            && tolower(*name as c_int) != tolower(*pointer as c_int))
            || (case_sensitive != 0 && *name != *pointer)
        {
            return c_false();
        }
        name = name.add(1);
        pointer = pointer.add(1);
    }

    let pointer_ended = (*pointer != 0) && (*pointer != b'/');
    let name_ended = *name != 0;
    if pointer_ended != name_ended {
        return c_false();
    }

    c_true()
}

unsafe fn pointer_encoded_length(mut string: *const u8) -> size_t {
    let mut length: size_t = 0;
    while *string != 0 {
        if *string == b'~' || *string == b'/' {
            length += 1;
        }
        string = string.add(1);
        length += 1;
    }
    length
}

unsafe fn encode_string_as_pointer(mut destination: *mut u8, mut source: *const u8) {
    while *source != 0 {
        if *source == b'/' {
            *destination = b'~';
            *destination.add(1) = b'1';
            destination = destination.add(1);
        } else if *source == b'~' {
            *destination = b'~';
            *destination.add(1) = b'0';
            destination = destination.add(1);
        } else {
            *destination = *source;
        }
        source = source.add(1);
        destination = destination.add(1);
    }
    *destination = 0;
}

unsafe fn get_array_item(array: *const cJSON, mut item: size_t) -> *mut cJSON {
    let mut child = if array.is_null() {
        ptr::null_mut()
    } else {
        (*array).child
    };
    while !child.is_null() && item > 0 {
        item -= 1;
        child = (*child).next;
    }
    child
}

unsafe fn decode_array_index_from_pointer(
    pointer: *const u8,
    index: *mut size_t,
) -> cJSON_bool {
    let mut parsed_index: size_t = 0;
    let mut position: size_t = 0;

    if *pointer == b'0' && (*pointer.add(1) != 0) && (*pointer.add(1) != b'/') {
        return c_false();
    }

    while *pointer.add(position) >= b'0' && *pointer.add(position) <= b'9' {
        parsed_index = (10 * parsed_index) + (*pointer.add(position) - b'0') as size_t;
        position += 1;
    }

    if *pointer.add(position) != 0 && *pointer.add(position) != b'/' {
        return c_false();
    }

    *index = parsed_index;
    c_true()
}

unsafe fn get_item_from_pointer(
    object: *mut cJSON,
    mut pointer: *const c_char,
    case_sensitive: cJSON_bool,
) -> *mut cJSON {
    let mut current_element = object;

    if pointer.is_null() {
        return ptr::null_mut();
    }

    while *pointer == b'/' as c_char && !current_element.is_null() {
        pointer = pointer.add(1);
        if cJSON_IsArray(current_element) != 0 {
            let mut index: size_t = 0;
            if decode_array_index_from_pointer(pointer as *const u8, &mut index) == 0 {
                return ptr::null_mut();
            }
            current_element = get_array_item(current_element, index);
        } else if cJSON_IsObject(current_element) != 0 {
            current_element = (*current_element).child;
            while !current_element.is_null()
                && compare_pointers(
                    (*current_element).string as *const u8,
                    pointer as *const u8,
                    case_sensitive,
                ) == 0
            {
                current_element = (*current_element).next;
            }
        } else {
            return ptr::null_mut();
        }

        while *pointer != 0 && *pointer != b'/' as c_char {
            pointer = pointer.add(1);
        }
    }

    current_element
}

/// Preserves the C `decode_pointer_inplace` `~1` quirk (`decoded_string[1] = '/'`).
unsafe fn decode_pointer_inplace(mut string: *mut u8) {
    if string.is_null() {
        return;
    }
    let mut decoded_string = string;
    while *string != 0 {
        if *string == b'~' {
            if *string.add(1) == b'0' {
                *decoded_string = b'~';
            } else if *string.add(1) == b'1' {
                // Exact C parity (known quirk): write to decoded_string[1], not [0].
                *decoded_string.add(1) = b'/';
            } else {
                return;
            }
            string = string.add(1);
        }
        decoded_string = decoded_string.add(1);
        string = string.add(1);
    }
    *decoded_string = 0;
}

unsafe fn detach_item_from_array(array: *mut cJSON, mut which: size_t) -> *mut cJSON {
    let mut c = (*array).child;
    while !c.is_null() && which > 0 {
        c = (*c).next;
        which -= 1;
    }
    if c.is_null() {
        return ptr::null_mut();
    }
    if c != (*array).child {
        (*(*c).prev).next = (*c).next;
    }
    if !(*c).next.is_null() {
        (*(*c).next).prev = (*c).prev;
    }
    if c == (*array).child {
        (*array).child = (*c).next;
    } else if (*c).next.is_null() {
        (*(*array).child).prev = (*c).prev;
    }
    (*c).prev = ptr::null_mut();
    (*c).next = ptr::null_mut();
    c
}

unsafe fn detach_path(
    object: *mut cJSON,
    path: *const u8,
    case_sensitive: cJSON_bool,
) -> *mut cJSON {
    let mut detached_item: *mut cJSON = ptr::null_mut();
    let parent_pointer = utils_strdup(path);
    if parent_pointer.is_null() {
        return ptr::null_mut();
    }

    let child_slash = libc::strrchr(parent_pointer as *const c_char, b'/' as c_int);
    if child_slash.is_null() {
        cJSON_free(parent_pointer as *mut _);
        return ptr::null_mut();
    }
    let child_pointer = child_slash as *mut u8;
    *child_pointer = 0;
    let child_pointer = child_pointer.add(1);

    let parent = get_item_from_pointer(object, parent_pointer as *const c_char, case_sensitive);
    decode_pointer_inplace(child_pointer);

    if cJSON_IsArray(parent) != 0 {
        let mut index: size_t = 0;
        if decode_array_index_from_pointer(child_pointer, &mut index) != 0 {
            detached_item = detach_item_from_array(parent, index);
        }
    } else if cJSON_IsObject(parent) != 0 {
        // C always uses case-insensitive detach here (even when case_sensitive).
        detached_item = cJSON_DetachItemFromObject(parent, child_pointer as *const c_char);
    }

    cJSON_free(parent_pointer as *mut _);
    detached_item
}

unsafe fn sort_list(list: *mut cJSON, case_sensitive: cJSON_bool) -> *mut cJSON {
    let mut first = list;
    let mut second = list;
    let mut current_item = list;
    let mut result = list;
    let mut result_tail: *mut cJSON = ptr::null_mut();

    if list.is_null() || (*list).next.is_null() {
        return result;
    }

    while !current_item.is_null()
        && !(*current_item).next.is_null()
        && compare_strings(
            (*current_item).string as *const u8,
            (*(*current_item).next).string as *const u8,
            case_sensitive,
        ) < 0
    {
        current_item = (*current_item).next;
    }
    if current_item.is_null() || (*current_item).next.is_null() {
        return result;
    }

    current_item = list;
    while !current_item.is_null() {
        second = (*second).next;
        current_item = (*current_item).next;
        if !current_item.is_null() {
            current_item = (*current_item).next;
        }
    }
    if !second.is_null() && !(*second).prev.is_null() {
        (*(*second).prev).next = ptr::null_mut();
        (*second).prev = ptr::null_mut();
    }

    first = sort_list(first, case_sensitive);
    second = sort_list(second, case_sensitive);
    result = ptr::null_mut();

    while !first.is_null() && !second.is_null() {
        let smaller = if compare_strings(
            (*first).string as *const u8,
            (*second).string as *const u8,
            case_sensitive,
        ) < 0
        {
            first
        } else {
            second
        };

        if result.is_null() {
            result_tail = smaller;
            result = smaller;
        } else {
            (*result_tail).next = smaller;
            (*smaller).prev = result_tail;
            result_tail = smaller;
        }

        if first == smaller {
            first = (*first).next;
        } else {
            second = (*second).next;
        }
    }

    if !first.is_null() {
        if result.is_null() {
            return first;
        }
        (*result_tail).next = first;
        (*first).prev = result_tail;
    }
    if !second.is_null() {
        if result.is_null() {
            return second;
        }
        (*result_tail).next = second;
        (*second).prev = result_tail;
    }

    result
}

unsafe fn sort_object(object: *mut cJSON, case_sensitive: cJSON_bool) {
    if object.is_null() {
        return;
    }
    (*object).child = sort_list((*object).child, case_sensitive);
}

unsafe fn compare_json(mut a: *mut cJSON, mut b: *mut cJSON, case_sensitive: cJSON_bool) -> cJSON_bool {
    if a.is_null() || b.is_null() || (((*a).type_0 & 0xFF) != ((*b).type_0 & 0xFF)) {
        return c_false();
    }
    match (*a).type_0 & 0xFF {
        x if x == cJSON_Number => {
            if (*a).valueint != (*b).valueint || compare_double((*a).valuedouble, (*b).valuedouble) == 0
            {
                c_false()
            } else {
                c_true()
            }
        }
        x if x == cJSON_String => {
            if strcmp((*a).valuestring, (*b).valuestring) != 0 {
                c_false()
            } else {
                c_true()
            }
        }
        x if x == cJSON_Array => {
            a = (*a).child;
            b = (*b).child;
            while !a.is_null() && !b.is_null() {
                if compare_json(a, b, case_sensitive) == 0 {
                    return c_false();
                }
                a = (*a).next;
                b = (*b).next;
            }
            if !a.is_null() || !b.is_null() {
                c_false()
            } else {
                c_true()
            }
        }
        x if x == cJSON_Object => {
            sort_object(a, case_sensitive);
            sort_object(b, case_sensitive);
            a = (*a).child;
            b = (*b).child;
            while !a.is_null() && !b.is_null() {
                if compare_strings(
                    (*a).string as *const u8,
                    (*b).string as *const u8,
                    case_sensitive,
                ) != 0
                {
                    return c_false();
                }
                if compare_json(a, b, case_sensitive) == 0 {
                    return c_false();
                }
                a = (*a).next;
                b = (*b).next;
            }
            if !a.is_null() || !b.is_null() {
                c_false()
            } else {
                c_true()
            }
        }
        _ => c_true(),
    }
}

unsafe fn insert_item_in_array(
    array: *mut cJSON,
    mut which: size_t,
    newitem: *mut cJSON,
) -> cJSON_bool {
    let mut child = (*array).child;
    while !child.is_null() && which > 0 {
        child = (*child).next;
        which -= 1;
    }
    if which > 0 {
        return c_false();
    }
    if child.is_null() {
        cJSON_AddItemToArray(array, newitem);
        return c_true();
    }

    (*newitem).next = child;
    (*newitem).prev = (*child).prev;
    (*child).prev = newitem;

    if child == (*array).child {
        (*array).child = newitem;
    } else {
        (*(*newitem).prev).next = newitem;
    }
    c_true()
}

unsafe fn get_object_item(
    object: *const cJSON,
    name: *const c_char,
    case_sensitive: cJSON_bool,
) -> *mut cJSON {
    if case_sensitive != 0 {
        cJSON_GetObjectItemCaseSensitive(object, name)
    } else {
        cJSON_GetObjectItem(object, name)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PatchOperation {
    Invalid,
    Add,
    Remove,
    Replace,
    Move,
    Copy,
    Test,
}

unsafe fn decode_patch_operation(patch: *const cJSON, case_sensitive: cJSON_bool) -> PatchOperation {
    let operation = get_object_item(patch, c"op".as_ptr(), case_sensitive);
    if cJSON_IsString(operation) == 0 {
        return PatchOperation::Invalid;
    }
    let vs = (*operation).valuestring;
    if strcmp(vs, c"add".as_ptr()) == 0 {
        PatchOperation::Add
    } else if strcmp(vs, c"remove".as_ptr()) == 0 {
        PatchOperation::Remove
    } else if strcmp(vs, c"replace".as_ptr()) == 0 {
        PatchOperation::Replace
    } else if strcmp(vs, c"move".as_ptr()) == 0 {
        PatchOperation::Move
    } else if strcmp(vs, c"copy".as_ptr()) == 0 {
        PatchOperation::Copy
    } else if strcmp(vs, c"test".as_ptr()) == 0 {
        PatchOperation::Test
    } else {
        PatchOperation::Invalid
    }
}

unsafe fn overwrite_item(root: *mut cJSON, replacement: cJSON) {
    if root.is_null() {
        return;
    }
    if !(*root).string.is_null() {
        cJSON_free((*root).string as *mut _);
    }
    if !(*root).valuestring.is_null() {
        cJSON_free((*root).valuestring as *mut _);
    }
    if !(*root).child.is_null() {
        cJSON_Delete((*root).child);
    }
    *root = replacement;
}

unsafe fn apply_patch(
    object: *mut cJSON,
    patch: *const cJSON,
    case_sensitive: cJSON_bool,
) -> c_int {
    let mut value: *mut cJSON = ptr::null_mut();
    let mut parent_pointer: *mut u8 = ptr::null_mut();

    let path = get_object_item(patch, c"path".as_ptr(), case_sensitive);
    if cJSON_IsString(path) == 0 {
        return 2;
    }

    let status = {
        let opcode = decode_patch_operation(patch, case_sensitive);
        if opcode == PatchOperation::Invalid {
            3
        } else if opcode == PatchOperation::Test {
            if compare_json(
                get_item_from_pointer(object, (*path).valuestring, case_sensitive),
                get_object_item(patch, c"value".as_ptr(), case_sensitive),
                case_sensitive,
            ) == 0
            {
                1
            } else {
                0
            }
        } else if *(*path).valuestring == 0 {
            // special case for replacing the root
            if opcode == PatchOperation::Remove {
                let invalid = cJSON {
                    next: ptr::null_mut(),
                    prev: ptr::null_mut(),
                    child: ptr::null_mut(),
                    type_0: cJSON_Invalid,
                    valuestring: ptr::null_mut(),
                    valueint: 0,
                    valuedouble: 0.0,
                    string: ptr::null_mut(),
                };
                overwrite_item(object, invalid);
                0
            } else if opcode == PatchOperation::Replace || opcode == PatchOperation::Add {
                value = get_object_item(patch, c"value".as_ptr(), case_sensitive);
                if value.is_null() {
                    7
                } else {
                    value = cJSON_Duplicate(value, 1);
                    if value.is_null() {
                        8
                    } else {
                        overwrite_item(object, *value);
                        cJSON_free(value as *mut _);
                        value = ptr::null_mut();
                        if !(*object).string.is_null() {
                            cJSON_free((*object).string as *mut _);
                            (*object).string = ptr::null_mut();
                        }
                        0
                    }
                }
            } else {
                // Empty path with MOVE/COPY: fall through like C (past the special-case block).
                apply_patch_body(
                    object,
                    patch,
                    path,
                    opcode,
                    case_sensitive,
                    &mut value,
                    &mut parent_pointer,
                )
            }
        } else {
            apply_patch_body(
                object,
                patch,
                path,
                opcode,
                case_sensitive,
                &mut value,
                &mut parent_pointer,
            )
        }
    };

    if !value.is_null() {
        cJSON_Delete(value);
    }
    if !parent_pointer.is_null() {
        cJSON_free(parent_pointer as *mut _);
    }
    status
}

/// Continuation of apply_patch after root special-cases (matches C control flow).
unsafe fn apply_patch_body(
    object: *mut cJSON,
    patch: *const cJSON,
    path: *mut cJSON,
    opcode: PatchOperation,
    case_sensitive: cJSON_bool,
    value: *mut *mut cJSON,
    parent_pointer: *mut *mut u8,
) -> c_int {
    let mut status: c_int = 0;

    if opcode == PatchOperation::Remove || opcode == PatchOperation::Replace {
        let old_item = detach_path(object, (*path).valuestring as *const u8, case_sensitive);
        if old_item.is_null() {
            return 13;
        }
        cJSON_Delete(old_item);
        if opcode == PatchOperation::Remove {
            return 0;
        }
    }

    if opcode == PatchOperation::Move || opcode == PatchOperation::Copy {
        let from = get_object_item(patch, c"from".as_ptr(), case_sensitive);
        if cJSON_IsString(from) == 0 {
            return 4;
        }
        if opcode == PatchOperation::Move {
            *value = detach_path(object, (*from).valuestring as *const u8, case_sensitive);
        }
        if opcode == PatchOperation::Copy {
            *value = get_item_from_pointer(object, (*from).valuestring, case_sensitive);
        }
        if (*value).is_null() {
            return 5;
        }
        if opcode == PatchOperation::Copy {
            *value = cJSON_Duplicate(*value, 1);
        }
        if (*value).is_null() {
            return 6;
        }
    } else {
        *value = get_object_item(patch, c"value".as_ptr(), case_sensitive);
        if (*value).is_null() {
            return 7;
        }
        *value = cJSON_Duplicate(*value, 1);
        if (*value).is_null() {
            return 8;
        }
    }

    *parent_pointer = utils_strdup((*path).valuestring as *const u8);
    let mut child_pointer: *mut u8 = ptr::null_mut();
    if !(*parent_pointer).is_null() {
        child_pointer = libc::strrchr(*parent_pointer as *const c_char, b'/' as c_int) as *mut u8;
    }
    if !child_pointer.is_null() {
        *child_pointer = 0;
        child_pointer = child_pointer.add(1);
    }
    let parent = get_item_from_pointer(object, *parent_pointer as *const c_char, case_sensitive);
    decode_pointer_inplace(child_pointer);

    if parent.is_null() || child_pointer.is_null() {
        status = 9;
    } else if cJSON_IsArray(parent) != 0 {
        if strcmp(child_pointer as *const c_char, c"-".as_ptr()) == 0 {
            cJSON_AddItemToArray(parent, *value);
            *value = ptr::null_mut();
        } else {
            let mut index: size_t = 0;
            if decode_array_index_from_pointer(child_pointer, &mut index) == 0 {
                status = 11;
            } else if insert_item_in_array(parent, index, *value) == 0 {
                status = 10;
            } else {
                *value = ptr::null_mut();
            }
        }
    } else if cJSON_IsObject(parent) != 0 {
        if case_sensitive != 0 {
            cJSON_DeleteItemFromObjectCaseSensitive(parent, child_pointer as *const c_char);
        } else {
            cJSON_DeleteItemFromObject(parent, child_pointer as *const c_char);
        }
        cJSON_AddItemToObject(parent, child_pointer as *const c_char, *value);
        *value = ptr::null_mut();
    } else {
        status = 9;
    }

    status
}

unsafe fn compose_patch(
    patches: *mut cJSON,
    operation: *const u8,
    path: *const u8,
    suffix: *const u8,
    value: *const cJSON,
) {
    if patches.is_null() || operation.is_null() || path.is_null() {
        return;
    }

    let patch = cJSON_CreateObject();
    if patch.is_null() {
        return;
    }
    cJSON_AddItemToObject(
        patch,
        c"op".as_ptr(),
        cJSON_CreateString(operation as *const c_char),
    );

    if suffix.is_null() {
        cJSON_AddItemToObject(
            patch,
            c"path".as_ptr(),
            cJSON_CreateString(path as *const c_char),
        );
    } else {
        let suffix_length = pointer_encoded_length(suffix);
        let path_length = strlen(path as *const c_char);
        let full_path = cJSON_malloc(path_length + suffix_length + 2) as *mut u8;
        // sprintf("%s/", path)
        ptr::copy_nonoverlapping(path, full_path, path_length);
        *full_path.add(path_length) = b'/';
        *full_path.add(path_length + 1) = 0;
        encode_string_as_pointer(full_path.add(path_length + 1), suffix);
        cJSON_AddItemToObject(
            patch,
            c"path".as_ptr(),
            cJSON_CreateString(full_path as *const c_char),
        );
        cJSON_free(full_path as *mut _);
    }

    if !value.is_null() {
        cJSON_AddItemToObject(patch, c"value".as_ptr(), cJSON_Duplicate(value, 1));
    }
    cJSON_AddItemToArray(patches, patch);
}

unsafe fn create_patches(
    patches: *mut cJSON,
    path: *const u8,
    from: *mut cJSON,
    to: *mut cJSON,
    case_sensitive: cJSON_bool,
) {
    if from.is_null() || to.is_null() {
        return;
    }

    if ((*from).type_0 & 0xFF) != ((*to).type_0 & 0xFF) {
        compose_patch(patches, c"replace".as_ptr() as *const u8, path, ptr::null(), to);
        return;
    }

    match (*from).type_0 & 0xFF {
        x if x == cJSON_Number => {
            if (*from).valueint != (*to).valueint
                || compare_double((*from).valuedouble, (*to).valuedouble) == 0
            {
                compose_patch(patches, c"replace".as_ptr() as *const u8, path, ptr::null(), to);
            }
        }
        x if x == cJSON_String => {
            if strcmp((*from).valuestring, (*to).valuestring) != 0 {
                compose_patch(patches, c"replace".as_ptr() as *const u8, path, ptr::null(), to);
            }
        }
        x if x == cJSON_Array => {
            let mut index: size_t = 0;
            let mut from_child = (*from).child;
            let mut to_child = (*to).child;
            let new_path = cJSON_malloc(strlen(path as *const c_char) + 20 + 2) as *mut u8;

            while !from_child.is_null() && !to_child.is_null() {
                if (index as u64) > ULONG_MAX {
                    cJSON_free(new_path as *mut _);
                    return;
                }
                libc::snprintf(
                    new_path as *mut c_char,
                    strlen(path as *const c_char) + 22,
                    c"%s/%lu".as_ptr(),
                    path as *const c_char,
                    index as libc::c_ulong,
                );
                create_patches(patches, new_path, from_child, to_child, case_sensitive);
                from_child = (*from_child).next;
                to_child = (*to_child).next;
                index += 1;
            }

            while !from_child.is_null() {
                if (index as u64) > ULONG_MAX {
                    cJSON_free(new_path as *mut _);
                    return;
                }
                libc::snprintf(
                    new_path as *mut c_char,
                    22,
                    c"%lu".as_ptr(),
                    index as libc::c_ulong,
                );
                compose_patch(
                    patches,
                    c"remove".as_ptr() as *const u8,
                    path,
                    new_path,
                    ptr::null(),
                );
                from_child = (*from_child).next;
            }
            while !to_child.is_null() {
                compose_patch(
                    patches,
                    c"add".as_ptr() as *const u8,
                    path,
                    c"-".as_ptr() as *const u8,
                    to_child,
                );
                to_child = (*to_child).next;
                index += 1;
            }
            cJSON_free(new_path as *mut _);
        }
        x if x == cJSON_Object => {
            sort_object(from, case_sensitive);
            sort_object(to, case_sensitive);
            let mut from_child = (*from).child;
            let mut to_child = (*to).child;
            while !from_child.is_null() || !to_child.is_null() {
                let diff = if from_child.is_null() {
                    1
                } else if to_child.is_null() {
                    -1
                } else {
                    compare_strings(
                        (*from_child).string as *const u8,
                        (*to_child).string as *const u8,
                        case_sensitive,
                    )
                };

                if diff == 0 {
                    let path_length = strlen(path as *const c_char);
                    let from_child_name_length =
                        pointer_encoded_length((*from_child).string as *const u8);
                    let new_path =
                        cJSON_malloc(path_length + from_child_name_length + 2) as *mut u8;
                    ptr::copy_nonoverlapping(path, new_path, path_length);
                    *new_path.add(path_length) = b'/';
                    *new_path.add(path_length + 1) = 0;
                    encode_string_as_pointer(
                        new_path.add(path_length + 1),
                        (*from_child).string as *const u8,
                    );
                    create_patches(patches, new_path, from_child, to_child, case_sensitive);
                    cJSON_free(new_path as *mut _);
                    from_child = (*from_child).next;
                    to_child = (*to_child).next;
                } else if diff < 0 {
                    compose_patch(
                        patches,
                        c"remove".as_ptr() as *const u8,
                        path,
                        (*from_child).string as *const u8,
                        ptr::null(),
                    );
                    from_child = (*from_child).next;
                } else {
                    compose_patch(
                        patches,
                        c"add".as_ptr() as *const u8,
                        path,
                        (*to_child).string as *const u8,
                        to_child,
                    );
                    to_child = (*to_child).next;
                }
            }
        }
        _ => {}
    }
}

unsafe fn merge_patch(
    mut target: *mut cJSON,
    patch: *const cJSON,
    case_sensitive: cJSON_bool,
) -> *mut cJSON {
    if cJSON_IsObject(patch) == 0 {
        let duplicate = cJSON_Duplicate(patch, 1);
        cJSON_Delete(target);
        return duplicate;
    }

    if cJSON_IsObject(target) == 0 {
        cJSON_Delete(target);
        target = cJSON_CreateObject();
    }

    let mut patch_child = (*patch).child;
    while !patch_child.is_null() {
        if cJSON_IsNull(patch_child) != 0 {
            if case_sensitive != 0 {
                cJSON_DeleteItemFromObjectCaseSensitive(target, (*patch_child).string);
            } else {
                cJSON_DeleteItemFromObject(target, (*patch_child).string);
            }
        } else {
            let replace_me = if case_sensitive != 0 {
                cJSON_DetachItemFromObjectCaseSensitive(target, (*patch_child).string)
            } else {
                cJSON_DetachItemFromObject(target, (*patch_child).string)
            };
            let replacement = merge_patch(replace_me, patch_child, case_sensitive);
            if replacement.is_null() {
                cJSON_Delete(target);
                return ptr::null_mut();
            }
            cJSON_AddItemToObject(target, (*patch_child).string, replacement);
        }
        patch_child = (*patch_child).next;
    }
    target
}

unsafe fn generate_merge_patch(
    from: *mut cJSON,
    to: *mut cJSON,
    case_sensitive: cJSON_bool,
) -> *mut cJSON {
    if to.is_null() {
        return cJSON_CreateNull();
    }
    if cJSON_IsObject(to) == 0 || cJSON_IsObject(from) == 0 {
        return cJSON_Duplicate(to, 1);
    }

    sort_object(from, case_sensitive);
    sort_object(to, case_sensitive);

    let mut from_child = (*from).child;
    let mut to_child = (*to).child;
    let patch = cJSON_CreateObject();
    if patch.is_null() {
        return ptr::null_mut();
    }

    while !from_child.is_null() || !to_child.is_null() {
        // C always uses strcmp here (even when case_sensitive is true).
        let diff = if !from_child.is_null() {
            if !to_child.is_null() {
                strcmp((*from_child).string, (*to_child).string)
            } else {
                -1
            }
        } else {
            1
        };

        if diff < 0 {
            cJSON_AddItemToObject(patch, (*from_child).string, cJSON_CreateNull());
            from_child = (*from_child).next;
        } else if diff > 0 {
            cJSON_AddItemToObject(patch, (*to_child).string, cJSON_Duplicate(to_child, 1));
            to_child = (*to_child).next;
        } else {
            if compare_json(from_child, to_child, case_sensitive) == 0 {
                // C always recurses via the non-case-sensitive public API.
                cJSON_AddItemToObject(
                    patch,
                    (*to_child).string,
                    cJSONUtils_GenerateMergePatch(from_child, to_child),
                );
            }
            from_child = (*from_child).next;
            to_child = (*to_child).next;
        }
    }

    if (*patch).child.is_null() {
        cJSON_Delete(patch);
        return ptr::null_mut();
    }
    patch
}

// --- Public C-ABI exports (cJSON_Utils.h) ---

#[no_mangle]
pub unsafe extern "C" fn cJSONUtils_FindPointerFromObjectTo(
    object: *const cJSON,
    target: *const cJSON,
) -> *mut c_char {
    let mut child_index: size_t = 0;

    if object.is_null() || target.is_null() {
        return ptr::null_mut();
    }

    if object as *const _ == target as *const _ {
        return utils_strdup(c"".as_ptr() as *const u8) as *mut c_char;
    }

    let mut current_child = (*object).child;
    while !current_child.is_null() {
        let target_pointer =
            cJSONUtils_FindPointerFromObjectTo(current_child, target) as *mut u8;
        if !target_pointer.is_null() {
            if cJSON_IsArray(object) != 0 {
                let full_pointer =
                    cJSON_malloc(strlen(target_pointer as *const c_char) + 20 + 2) as *mut u8;
                if (child_index as u64) > ULONG_MAX {
                    cJSON_free(target_pointer as *mut _);
                    cJSON_free(full_pointer as *mut _);
                    return ptr::null_mut();
                }
                libc::snprintf(
                    full_pointer as *mut c_char,
                    strlen(target_pointer as *const c_char) + 22,
                    c"/%lu%s".as_ptr(),
                    child_index as libc::c_ulong,
                    target_pointer as *const c_char,
                );
                cJSON_free(target_pointer as *mut _);
                return full_pointer as *mut c_char;
            }

            if cJSON_IsObject(object) != 0 {
                let full_pointer = cJSON_malloc(
                    strlen(target_pointer as *const c_char)
                        + pointer_encoded_length((*current_child).string as *const u8)
                        + 2,
                ) as *mut u8;
                *full_pointer = b'/';
                encode_string_as_pointer(full_pointer.add(1), (*current_child).string as *const u8);
                let encoded_len = strlen(full_pointer as *const c_char);
                libc::strcat(full_pointer as *mut c_char, target_pointer as *const c_char);
                let _ = encoded_len;
                cJSON_free(target_pointer as *mut _);
                return full_pointer as *mut c_char;
            }

            cJSON_free(target_pointer as *mut _);
            return ptr::null_mut();
        }
        current_child = (*current_child).next;
        child_index += 1;
    }

    ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn cJSONUtils_GetPointer(
    object: *mut cJSON,
    pointer: *const c_char,
) -> *mut cJSON {
    get_item_from_pointer(object, pointer, c_false())
}

#[no_mangle]
pub unsafe extern "C" fn cJSONUtils_GetPointerCaseSensitive(
    object: *mut cJSON,
    pointer: *const c_char,
) -> *mut cJSON {
    get_item_from_pointer(object, pointer, c_true())
}

#[no_mangle]
pub unsafe extern "C" fn cJSONUtils_ApplyPatches(
    object: *mut cJSON,
    patches: *const cJSON,
) -> c_int {
    if cJSON_IsArray(patches) == 0 {
        return 1;
    }
    let mut current_patch = if patches.is_null() {
        ptr::null()
    } else {
        (*patches).child as *const cJSON
    };
    while !current_patch.is_null() {
        let status = apply_patch(object, current_patch, c_false());
        if status != 0 {
            return status;
        }
        current_patch = (*current_patch).next;
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn cJSONUtils_ApplyPatchesCaseSensitive(
    object: *mut cJSON,
    patches: *const cJSON,
) -> c_int {
    if cJSON_IsArray(patches) == 0 {
        return 1;
    }
    let mut current_patch = if patches.is_null() {
        ptr::null()
    } else {
        (*patches).child as *const cJSON
    };
    while !current_patch.is_null() {
        let status = apply_patch(object, current_patch, c_true());
        if status != 0 {
            return status;
        }
        current_patch = (*current_patch).next;
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn cJSONUtils_AddPatchToArray(
    array: *mut cJSON,
    operation: *const c_char,
    path: *const c_char,
    value: *const cJSON,
) {
    compose_patch(
        array,
        operation as *const u8,
        path as *const u8,
        ptr::null(),
        value,
    );
}

#[no_mangle]
pub unsafe extern "C" fn cJSONUtils_GeneratePatches(
    from: *mut cJSON,
    to: *mut cJSON,
) -> *mut cJSON {
    if from.is_null() || to.is_null() {
        return ptr::null_mut();
    }
    let patches = cJSON_CreateArray();
    create_patches(patches, c"".as_ptr() as *const u8, from, to, c_false());
    patches
}

#[no_mangle]
pub unsafe extern "C" fn cJSONUtils_GeneratePatchesCaseSensitive(
    from: *mut cJSON,
    to: *mut cJSON,
) -> *mut cJSON {
    if from.is_null() || to.is_null() {
        return ptr::null_mut();
    }
    let patches = cJSON_CreateArray();
    create_patches(patches, c"".as_ptr() as *const u8, from, to, c_true());
    patches
}

#[no_mangle]
pub unsafe extern "C" fn cJSONUtils_SortObject(object: *mut cJSON) {
    sort_object(object, c_false());
}

#[no_mangle]
pub unsafe extern "C" fn cJSONUtils_SortObjectCaseSensitive(object: *mut cJSON) {
    sort_object(object, c_true());
}

#[no_mangle]
pub unsafe extern "C" fn cJSONUtils_MergePatch(
    target: *mut cJSON,
    patch: *const cJSON,
) -> *mut cJSON {
    merge_patch(target, patch, c_false())
}

#[no_mangle]
pub unsafe extern "C" fn cJSONUtils_MergePatchCaseSensitive(
    target: *mut cJSON,
    patch: *const cJSON,
) -> *mut cJSON {
    merge_patch(target, patch, c_true())
}

#[no_mangle]
pub unsafe extern "C" fn cJSONUtils_GenerateMergePatch(
    from: *mut cJSON,
    to: *mut cJSON,
) -> *mut cJSON {
    generate_merge_patch(from, to, c_false())
}

#[no_mangle]
pub unsafe extern "C" fn cJSONUtils_GenerateMergePatchCaseSensitive(
    from: *mut cJSON,
    to: *mut cJSON,
) -> *mut cJSON {
    generate_merge_patch(from, to, c_true())
}
