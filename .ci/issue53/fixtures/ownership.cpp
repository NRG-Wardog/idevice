#include <cassert>
#include <cstdio>
#include <idevice++/house_arrest.hpp>

static int released_provider = 0;
static int released_house = 0;
static bool fail_vend = false;
static IdeviceFfiError error = {13, 0, "controlled failure"};
extern "C" {
void idevice_free(IdeviceHandle*) {}
void idevice_provider_free(IdeviceProviderHandle* p) { assert(p->alive); p->alive = 0; ++released_provider; }
void idevice_error_free(IdeviceFfiError*) {}
void house_arrest_client_free(HouseArrestClientHandle* h) { assert(h->alive); h->alive = 0; ++released_house; }
IdeviceFfiError *house_arrest_client_connect(IdeviceProviderHandle* p, HouseArrestClientHandle**) { assert(p->alive); return &error; }
IdeviceFfiError *house_arrest_client_connect_rsd(AdapterHandle*, RsdHandshakeHandle*, HouseArrestClientHandle**) { return &error; }
IdeviceFfiError *house_arrest_client_new(IdeviceHandle*, HouseArrestClientHandle**) { return &error; }
IdeviceFfiError *house_arrest_vend_container(HouseArrestClientHandle* h, const char*, AfcClientHandle** out) {
    if (!h) return &error;
    assert(h->alive); h->alive = 0; ++released_house;
    static AfcClientHandle afc{1};
    if (fail_vend) return &error;
    *out = &afc;
    return nullptr;
}
IdeviceFfiError *house_arrest_vend_documents(HouseArrestClientHandle* h, const char* b, AfcClientHandle** out) {
    return house_arrest_vend_container(h, b, out);
}
}
int main() {
    using namespace IdeviceFFI;
    IdeviceProviderHandle p{1};
    {
        auto provider = Provider::adopt(&p);
        auto result = HouseArrest::connect(provider);
        assert(result.is_err());
        assert(provider.raw() == &p && p.alive);
    }
    assert(released_provider == 1);
    for (bool failure : {false, true}) {
        for (bool documents : {false, true}) {
            HouseArrestClientHandle h{1};
            const int before = released_house;
            {
                auto house = HouseArrest::adopt(&h);
                fail_vend = failure;
                auto result = documents ? house.vend_documents("fixture") : house.vend_container("fixture");
                assert(result.is_err() == failure);
                assert(house.raw() == nullptr);
                assert(!h.alive);
                auto repeated = house.vend_documents("fixture");
                assert(repeated.is_err());
            }
            assert(released_house == before + 1);
        }
    }
    std::puts("OWNERSHIP_PASS provider_borrow=1 vend_success_failure_repeat=4");
}
